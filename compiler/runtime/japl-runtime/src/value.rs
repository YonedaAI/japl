//! Runtime value representation for JAPL.
//!
//! All JAPL values at runtime are represented by the `Value` enum. Values are
//! immutable by design -- this enables safe cross-process sharing via reference
//! counting and eliminates the need for write barriers in the GC.

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::error::CrashReason;

/// Process identifier type.
pub type ProcessId = u64;

/// A runtime value in the JAPL system.
///
/// All variants are immutable. Shared data uses `Arc` for thread-safe
/// reference counting, which is safe because values are never mutated
/// after construction.
#[derive(Debug, Clone)]
pub enum Value {
    /// 64-bit signed integer.
    Int(i64),
    /// 64-bit IEEE 754 float.
    Float(f64),
    /// Boolean value.
    Bool(bool),
    /// Immutable string (reference counted).
    String(Arc<str>),
    /// Raw byte sequence (reference counted).
    Bytes(Arc<[u8]>),
    /// Unit value (empty tuple).
    Unit,
    /// Fixed-size heterogeneous tuple.
    Tuple(Vec<Value>),
    /// Named record with field-value pairs.
    Record(Vec<(Arc<str>, Value)>),
    /// Persistent linked list.
    List(Vec<Value>),
    /// Tagged union variant (constructor name + fields).
    Constructor(Arc<str>, Vec<Value>),
    /// Process identifier.
    Pid(ProcessId),
    /// Function closure.
    Closure(Arc<Closure>),
    /// Linear resource handle.
    Resource(ResourceHandle),
    /// Crash reason (for exit signals).
    CrashReason(CrashReason),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Constructor(na, fa), Value::Constructor(nb, fb)) => na == nb && fa == fb,
            (Value::Pid(a), Value::Pid(b)) => a == b,
            (Value::CrashReason(a), Value::CrashReason(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Bytes(b) => write!(f, "<<{} bytes>>", b.len()),
            Value::Unit => write!(f, "()"),
            Value::Tuple(elems) => {
                write!(f, "(")?;
                for (i, v) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            Value::Record(fields) => {
                write!(f, "{{ ")?;
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {}", name, val)?;
                }
                write!(f, " }}")
            }
            Value::List(elems) => {
                write!(f, "[")?;
                for (i, v) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Constructor(name, fields) => {
                write!(f, "{}", name)?;
                if !fields.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Pid(pid) => write!(f, "<pid:{}>", pid),
            Value::Closure(c) => write!(f, "<closure/{}>", c.arity),
            Value::Resource(r) => write!(f, "<resource:{}:{}>", r.resource_type, r.id),
            Value::CrashReason(reason) => write!(f, "<crash:{}>", reason),
        }
    }
}

/// A function pointer type for closures.
///
/// In the runtime, closures carry a function pointer along with captured
/// environment values. The function pointer takes the captured values and
/// arguments and produces a result.
pub type FnPtr = fn(&[Value], &[Value]) -> Value;

/// A closure value capturing environment variables and a code pointer.
#[derive(Debug)]
pub struct Closure {
    /// Number of parameters the closure expects.
    pub arity: usize,
    /// Captured environment values (from lexical scope).
    pub captured: Vec<Value>,
    /// Pointer to the compiled function code.
    pub code: FnPtr,
}

/// A linear resource handle.
///
/// Resources represent external entities (file handles, sockets, etc.)
/// that must be explicitly consumed. The `consumed` flag tracks whether
/// the resource has been used, enforcing linear usage at runtime.
#[derive(Debug)]
pub struct ResourceHandle {
    /// Unique identifier for this resource instance.
    pub id: u64,
    /// Type name for debugging and introspection.
    pub resource_type: Arc<str>,
    /// Whether this resource has been consumed (linear enforcement).
    pub consumed: AtomicBool,
}

impl Clone for ResourceHandle {
    fn clone(&self) -> Self {
        ResourceHandle {
            id: self.id,
            resource_type: Arc::clone(&self.resource_type),
            consumed: AtomicBool::new(
                self.consumed
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

/// Convert a `Value` to its string representation (the `show` function).
pub fn show(val: &Value) -> String {
    format!("{}", val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_int() {
        let v = Value::Int(42);
        assert_eq!(format!("{}", v), "42");
        assert_eq!(v, Value::Int(42));
    }

    #[test]
    fn test_value_string() {
        let v = Value::String(Arc::from("hello"));
        assert_eq!(format!("{}", v), "\"hello\"");
    }

    #[test]
    fn test_value_tuple() {
        let v = Value::Tuple(vec![Value::Int(1), Value::Bool(true)]);
        assert_eq!(format!("{}", v), "(1, true)");
    }

    #[test]
    fn test_value_list() {
        let v = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(format!("{}", v), "[1, 2, 3]");
    }

    #[test]
    fn test_value_constructor() {
        let v = Value::Constructor(Arc::from("Some"), vec![Value::Int(42)]);
        assert_eq!(format!("{}", v), "Some(42)");
    }

    #[test]
    fn test_value_record() {
        let v = Value::Record(vec![
            (Arc::from("x"), Value::Int(1)),
            (Arc::from("y"), Value::Int(2)),
        ]);
        assert_eq!(format!("{}", v), "{ x = 1, y = 2 }");
    }

    #[test]
    fn test_value_unit() {
        assert_eq!(Value::Unit, Value::Unit);
        assert_eq!(format!("{}", Value::Unit), "()");
    }

    #[test]
    fn test_value_pid() {
        let v = Value::Pid(42);
        assert_eq!(format!("{}", v), "<pid:42>");
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(1), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Int(2));
        assert_ne!(Value::Int(1), Value::Bool(true));
    }
}

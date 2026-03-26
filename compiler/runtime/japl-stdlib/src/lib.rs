//! JAPL Standard Library
//!
//! Core functions available to all JAPL programs. These are the built-in
//! operations that the compiler can call directly.

use std::sync::Arc;

use japl_runtime::value::Value;
use japl_runtime::ProcessId;

// ---------------------------------------------------------------------------
// I/O functions
// ---------------------------------------------------------------------------

/// Print a string to stdout (no trailing newline).
pub fn print(s: &str) {
    std::io::Write::write_all(&mut std::io::stdout(), s.as_bytes())
        .expect("failed to write to stdout");
}

/// Print a string to stdout followed by a newline.
pub fn println(s: &str) {
    std::io::Write::write_all(&mut std::io::stdout(), s.as_bytes())
        .expect("failed to write to stdout");
    std::io::Write::write_all(&mut std::io::stdout(), b"\n")
        .expect("failed to write to stdout");
}

// ---------------------------------------------------------------------------
// Value display
// ---------------------------------------------------------------------------

/// Convert any value to its string representation.
pub fn show(val: &Value) -> String {
    format!("{}", val)
}

// ---------------------------------------------------------------------------
// List operations
// ---------------------------------------------------------------------------

/// Apply a function to each element of a list, returning a new list.
pub fn list_map(list: &Value, f: impl Fn(&Value) -> Value) -> Value {
    match list {
        Value::List(elems) => {
            let mapped: Vec<Value> = elems.iter().map(|e| f(e)).collect();
            Value::List(mapped)
        }
        _ => Value::List(Vec::new()),
    }
}

/// Filter a list, keeping only elements for which the predicate returns true.
pub fn list_filter(list: &Value, pred: impl Fn(&Value) -> bool) -> Value {
    match list {
        Value::List(elems) => {
            let filtered: Vec<Value> = elems.iter().filter(|e| pred(e)).cloned().collect();
            Value::List(filtered)
        }
        _ => Value::List(Vec::new()),
    }
}

/// Left fold over a list: accumulate a result starting from `init`.
pub fn list_fold(list: &Value, init: Value, f: impl Fn(Value, &Value) -> Value) -> Value {
    match list {
        Value::List(elems) => {
            let mut acc = init;
            for elem in elems.iter() {
                acc = f(acc, elem);
            }
            acc
        }
        _ => init,
    }
}

/// Return the length of a list.
pub fn list_length(list: &Value) -> Value {
    match list {
        Value::List(elems) => Value::Int(elems.len() as i64),
        _ => Value::Int(0),
    }
}

// ---------------------------------------------------------------------------
// String operations
// ---------------------------------------------------------------------------

/// Concatenate two strings.
pub fn string_concat(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::String(sa), Value::String(sb)) => {
            let mut result = String::from(&**sa);
            result.push_str(sb);
            Value::String(Arc::from(result.as_str()))
        }
        _ => Value::String(Arc::from("")),
    }
}

/// Return the length of a string (in bytes).
pub fn string_length(s: &Value) -> Value {
    match s {
        Value::String(s) => Value::Int(s.len() as i64),
        _ => Value::Int(0),
    }
}

// ---------------------------------------------------------------------------
// Conversion functions
// ---------------------------------------------------------------------------

/// Convert an integer to its string representation.
pub fn int_to_string(n: &Value) -> Value {
    match n {
        Value::Int(i) => Value::String(Arc::from(i.to_string().as_str())),
        _ => Value::String(Arc::from("")),
    }
}

// ---------------------------------------------------------------------------
// Process operations (delegate to the runtime scheduler)
// ---------------------------------------------------------------------------

/// Get the current process's PID.
///
/// In actual compiled JAPL code, this reads from the process context.
/// This function is a placeholder that the code generator wires up.
pub fn process_self(pid: ProcessId) -> Value {
    Value::Pid(pid)
}

/// Spawn a new process. Returns the PID of the new process.
///
/// In actual compiled JAPL code, this calls into the scheduler.
/// This function provides the interface that the code generator targets.
pub fn process_spawn(scheduler: &japl_runtime::Scheduler, f: impl FnOnce(&mut japl_runtime::process::ProcessContext) + Send + 'static) -> Value {
    let pid = scheduler.spawn_fn(f);
    Value::Pid(pid)
}

/// Send a message to a process.
pub fn process_send(scheduler: &japl_runtime::Scheduler, pid: ProcessId, value: Value) {
    let _ = scheduler.send_message(pid, value);
}

/// List all process IDs.
pub fn process_list(scheduler: &japl_runtime::Scheduler) -> Value {
    let pids: Vec<Value> = scheduler
        .process_list()
        .into_iter()
        .map(Value::Pid)
        .collect();
    Value::List(pids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_show_int() {
        assert_eq!(show(&Value::Int(42)), "42");
    }

    #[test]
    fn test_show_string() {
        assert_eq!(show(&Value::String(Arc::from("hello"))), "\"hello\"");
    }

    #[test]
    fn test_list_map() {
        let list = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = list_map(&list, |v| match v {
            Value::Int(n) => Value::Int(n * 2),
            _ => v.clone(),
        });
        assert_eq!(
            result,
            Value::List(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
        );
    }

    #[test]
    fn test_list_filter() {
        let list = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)]);
        let result = list_filter(&list, |v| match v {
            Value::Int(n) => n % 2 == 0,
            _ => false,
        });
        assert_eq!(
            result,
            Value::List(vec![Value::Int(2), Value::Int(4)])
        );
    }

    #[test]
    fn test_list_fold() {
        let list = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = list_fold(&list, Value::Int(0), |acc, v| match (&acc, v) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
            _ => acc,
        });
        assert_eq!(result, Value::Int(6));
    }

    #[test]
    fn test_list_length() {
        let list = Value::List(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(list_length(&list), Value::Int(2));
    }

    #[test]
    fn test_string_concat() {
        let a = Value::String(Arc::from("hello"));
        let b = Value::String(Arc::from(" world"));
        let result = string_concat(&a, &b);
        assert_eq!(result, Value::String(Arc::from("hello world")));
    }

    #[test]
    fn test_string_length() {
        let s = Value::String(Arc::from("hello"));
        assert_eq!(string_length(&s), Value::Int(5));
    }

    #[test]
    fn test_int_to_string() {
        let n = Value::Int(42);
        assert_eq!(int_to_string(&n), Value::String(Arc::from("42")));
    }

    #[test]
    fn test_process_self() {
        assert_eq!(process_self(7), Value::Pid(7));
    }
}

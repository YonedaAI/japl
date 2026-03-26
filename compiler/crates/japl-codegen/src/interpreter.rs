//! Tree-walking interpreter for the JAPL IR.

use std::collections::HashMap;
use std::sync::Arc;

use japl_ir::{IrBinOp, IrExpr, IrPattern, IrProgram, IrUnaryOp};
use japl_runtime::Value;

use crate::env::Environment;

/// Errors that can occur during interpretation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum InterpreterError {
    #[error("undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("not a function: {0}")]
    NotAFunction(String),
    #[error("arity mismatch: expected {expected}, got {got}")]
    ArityMismatch { expected: usize, got: usize },
    #[error("type error: {0}")]
    TypeError(String),
    #[error("no matching pattern")]
    NoMatchingPattern,
    #[error("division by zero")]
    DivisionByZero,
    #[error("no main function found")]
    NoMainFunction,
    #[error("runtime error: {0}")]
    Runtime(String),
}

/// A callable function value stored in the interpreter.
#[derive(Debug, Clone)]
enum Callable {
    /// A user-defined function.
    UserFn(Vec<String>, IrExpr),
    /// A closure (captured environment + params + body).
    Closure(Environment, Vec<String>, IrExpr),
    /// A built-in stdlib function.
    Builtin(String),
    /// A constructor function.
    Constructor(String, usize),
}

/// The tree-walking interpreter.
pub struct Interpreter {
    env: Environment,
    /// Global function table.
    functions: HashMap<String, Callable>,
    /// Captured output (for testing).
    output: Vec<String>,
    /// Whether to capture output instead of printing.
    capture_output: bool,
}

impl Interpreter {
    /// Create a new interpreter.
    pub fn new() -> Self {
        let mut interp = Interpreter {
            env: Environment::new(),
            functions: HashMap::new(),
            output: Vec::new(),
            capture_output: false,
        };
        interp.register_builtins();
        interp
    }

    /// Create an interpreter that captures output (for testing).
    pub fn new_capturing() -> Self {
        let mut interp = Self::new();
        interp.capture_output = true;
        interp
    }

    /// Get captured output lines.
    pub fn get_output(&self) -> &[String] {
        &self.output
    }

    /// Register built-in functions.
    fn register_builtins(&mut self) {
        let builtins = [
            "println",
            "print",
            "show",
            "int_to_string",
            "float_to_string",
            "string_length",
            "string_concat",
            "list_length",
            "list_head",
            "list_tail",
            "list_append",
            "list_map",
            "list_filter",
            "list_fold",
            "__record_update",
        ];
        for name in &builtins {
            self.functions
                .insert(name.to_string(), Callable::Builtin(name.to_string()));
        }
    }

    /// Load an IR program, registering all functions and constructors.
    pub fn load_program(&mut self, program: &IrProgram) {
        // Register type constructors
        for type_def in &program.type_defs {
            for variant in &type_def.variants {
                if variant.arity == 0 {
                    // Nullary constructor: bind as a value in the environment
                    self.env.bind(
                        variant.name.clone(),
                        Value::Constructor(Arc::from(variant.name.as_str()), vec![]),
                    );
                } else {
                    // N-ary constructor: register as a callable
                    self.functions.insert(
                        variant.name.clone(),
                        Callable::Constructor(variant.name.clone(), variant.arity),
                    );
                }
            }
        }

        // Register user-defined functions
        for fn_def in &program.functions {
            self.functions.insert(
                fn_def.name.clone(),
                Callable::UserFn(fn_def.params.clone(), fn_def.body.clone()),
            );
        }
    }

    /// Run the program by calling the `main` function.
    pub fn run_program(&mut self, program: &IrProgram) -> Result<Value, InterpreterError> {
        self.load_program(program);

        if !self.functions.contains_key("main") {
            return Err(InterpreterError::NoMainFunction);
        }

        self.call_function("main", &[])
    }

    /// Interpret an IR expression and return its value.
    pub fn interpret(&mut self, expr: &IrExpr) -> Result<Value, InterpreterError> {
        match expr {
            IrExpr::Lit(val) => Ok(val.clone()),

            IrExpr::Var(name) => {
                // First check the environment
                if let Some(val) = self.env.lookup(name) {
                    return Ok(val.clone());
                }
                // Then check if it's a function (wrap as closure)
                if self.functions.contains_key(name) {
                    // Return a sentinel that App can dispatch
                    return Ok(Value::String(Arc::from(format!("__fn:{}", name).as_str())));
                }
                Err(InterpreterError::UndefinedVariable(name.clone()))
            }

            IrExpr::Let(name, value_expr, body_expr) => {
                let value = self.interpret(value_expr)?;
                self.env.push_scope();
                self.env.bind(name.clone(), value);
                let result = self.interpret(body_expr);
                self.env.pop_scope();
                result
            }

            IrExpr::App(func_expr, arg_exprs) => {
                let args: Vec<Value> = arg_exprs
                    .iter()
                    .map(|a| self.interpret(a))
                    .collect::<Result<_, _>>()?;

                // Check if the function expression is a variable that names a known function
                if let IrExpr::Var(name) = func_expr.as_ref() {
                    // Check environment first for closures
                    if let Some(val) = self.env.lookup(name).cloned() {
                        return self.call_value(&val, &args);
                    }
                    if self.functions.contains_key(name) {
                        return self.call_function(name, &args);
                    }
                    return Err(InterpreterError::UndefinedVariable(name.clone()));
                }

                let func_val = self.interpret(func_expr)?;
                self.call_value(&func_val, &args)
            }

            IrExpr::Lambda(params, body) => {
                let captured_env = self.env.snapshot();
                let callable = Callable::Closure(captured_env, params.clone(), *body.clone());
                // Store the callable and return a reference value
                let id = format!("__lambda_{}", self.functions.len());
                self.functions.insert(id.clone(), callable);
                Ok(Value::String(Arc::from(format!("__fn:{}", id).as_str())))
            }

            IrExpr::If(cond_expr, then_expr, else_expr) => {
                let cond = self.interpret(cond_expr)?;
                match cond {
                    Value::Bool(true) => self.interpret(then_expr),
                    Value::Bool(false) => self.interpret(else_expr),
                    _ => Err(InterpreterError::TypeError(
                        "if condition must be Bool".to_string(),
                    )),
                }
            }

            IrExpr::Match(scrutinee_expr, arms) => {
                let scrutinee = self.interpret(scrutinee_expr)?;
                for (pattern, guard, body) in arms {
                    let mut bindings = HashMap::new();
                    if match_pattern(pattern, &scrutinee, &mut bindings) {
                        self.env.push_scope();
                        for (name, val) in &bindings {
                            self.env.bind(name.clone(), val.clone());
                        }
                        // Check guard if present
                        if let Some(guard_expr) = guard {
                            let guard_val = self.interpret(guard_expr)?;
                            if guard_val != Value::Bool(true) {
                                self.env.pop_scope();
                                continue;
                            }
                        }
                        let result = self.interpret(body);
                        self.env.pop_scope();
                        return result;
                    }
                }
                Err(InterpreterError::NoMatchingPattern)
            }

            IrExpr::BinOp(op, lhs_expr, rhs_expr) => {
                let lhs = self.interpret(lhs_expr)?;
                let rhs = self.interpret(rhs_expr)?;
                eval_binop(*op, &lhs, &rhs)
            }

            IrExpr::UnaryOp(op, inner_expr) => {
                let val = self.interpret(inner_expr)?;
                eval_unaryop(*op, &val)
            }

            IrExpr::Record(fields) => {
                let ir_fields: Vec<(Arc<str>, Value)> = fields
                    .iter()
                    .map(|(name, field_expr)| {
                        let val = self.interpret(field_expr)?;
                        Ok((Arc::from(name.as_str()), val))
                    })
                    .collect::<Result<_, InterpreterError>>()?;
                Ok(Value::Record(ir_fields))
            }

            IrExpr::FieldAccess(record_expr, field) => {
                let val = self.interpret(record_expr)?;
                match val {
                    Value::Record(fields) => {
                        for (name, v) in &fields {
                            if name.as_ref() == field.as_str() {
                                return Ok(v.clone());
                            }
                        }
                        Err(InterpreterError::Runtime(format!(
                            "field '{}' not found in record",
                            field
                        )))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "field access on non-record".to_string(),
                    )),
                }
            }

            IrExpr::List(elems) => {
                let vals: Vec<Value> = elems
                    .iter()
                    .map(|e| self.interpret(e))
                    .collect::<Result<_, _>>()?;
                Ok(Value::List(vals))
            }

            IrExpr::Tuple(elems) => {
                let vals: Vec<Value> = elems
                    .iter()
                    .map(|e| self.interpret(e))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Tuple(vals))
            }

            IrExpr::Constructor(name, arg_exprs) => {
                let args: Vec<Value> = arg_exprs
                    .iter()
                    .map(|a| self.interpret(a))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Constructor(Arc::from(name.as_str()), args))
            }

            IrExpr::Block(exprs) => {
                let mut result = Value::Unit;
                for block_expr in exprs {
                    result = self.interpret(block_expr)?;
                }
                Ok(result)
            }

            IrExpr::Concat(lhs_expr, rhs_expr) => {
                let lhs = self.interpret(lhs_expr)?;
                let rhs = self.interpret(rhs_expr)?;
                match (&lhs, &rhs) {
                    (Value::String(a), Value::String(b)) => {
                        let mut result = String::from(&**a);
                        result.push_str(b);
                        Ok(Value::String(Arc::from(result.as_str())))
                    }
                    _ => Err(InterpreterError::TypeError(format!(
                        "cannot concatenate {:?} and {:?}",
                        lhs, rhs
                    ))),
                }
            }

            IrExpr::Pipeline(lhs_expr, rhs_expr) => {
                // x |> f  ==>  f(x)
                let arg = self.interpret(lhs_expr)?;
                // Check if rhs is a named function
                if let IrExpr::Var(name) = rhs_expr.as_ref() {
                    if let Some(val) = self.env.lookup(name).cloned() {
                        return self.call_value(&val, &[arg]);
                    }
                    if self.functions.contains_key(name) {
                        return self.call_function(name, &[arg]);
                    }
                    return Err(InterpreterError::UndefinedVariable(name.clone()));
                }
                let func_val = self.interpret(rhs_expr)?;
                self.call_value(&func_val, &[arg])
            }
        }
    }

    /// Call a named function.
    fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value, InterpreterError> {
        let callable = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| InterpreterError::UndefinedVariable(name.to_string()))?;
        self.call_callable(&callable, args)
    }

    /// Call a value as a function.
    fn call_value(&mut self, val: &Value, args: &[Value]) -> Result<Value, InterpreterError> {
        match val {
            Value::String(s) if s.starts_with("__fn:") => {
                let fn_name = &s[5..];
                self.call_function(fn_name, args)
            }
            _ => Err(InterpreterError::NotAFunction(format!("{:?}", val))),
        }
    }

    /// Call a callable with arguments.
    fn call_callable(
        &mut self,
        callable: &Callable,
        args: &[Value],
    ) -> Result<Value, InterpreterError> {
        match callable {
            Callable::UserFn(params, body) => {
                if params.len() != args.len() {
                    return Err(InterpreterError::ArityMismatch {
                        expected: params.len(),
                        got: args.len(),
                    });
                }
                let params = params.clone();
                let body = body.clone();
                self.env.push_scope();
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.env.bind(param.clone(), arg.clone());
                }
                let result = self.interpret(&body);
                self.env.pop_scope();
                result
            }

            Callable::Closure(captured_env, params, body) => {
                if params.len() != args.len() {
                    return Err(InterpreterError::ArityMismatch {
                        expected: params.len(),
                        got: args.len(),
                    });
                }
                // Save current environment, switch to captured
                let saved_env = std::mem::replace(&mut self.env, captured_env.clone());
                self.env.push_scope();
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.env.bind(param.clone(), arg.clone());
                }
                let body = body.clone();
                let result = self.interpret(&body);
                self.env = saved_env;
                result
            }

            Callable::Builtin(name) => self.call_builtin(name, args),

            Callable::Constructor(name, arity) => {
                if args.len() != *arity {
                    return Err(InterpreterError::ArityMismatch {
                        expected: *arity,
                        got: args.len(),
                    });
                }
                Ok(Value::Constructor(
                    Arc::from(name.as_str()),
                    args.to_vec(),
                ))
            }
        }
    }

    /// Dispatch a built-in function call.
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, InterpreterError> {
        match name {
            "println" => {
                let s = match args.first() {
                    Some(Value::String(s)) => s.to_string(),
                    Some(val) => format!("{}", val),
                    None => String::new(),
                };
                if self.capture_output {
                    self.output.push(s);
                } else {
                    japl_stdlib::println(&s);
                }
                Ok(Value::Unit)
            }

            "print" => {
                let s = match args.first() {
                    Some(Value::String(s)) => s.to_string(),
                    Some(val) => format!("{}", val),
                    None => String::new(),
                };
                if self.capture_output {
                    self.output.push(s);
                } else {
                    japl_stdlib::print(&s);
                }
                Ok(Value::Unit)
            }

            "show" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                let s = match val {
                    Value::String(s) => s.to_string(),
                    other => format!("{}", other),
                };
                Ok(Value::String(Arc::from(s.as_str())))
            }

            "int_to_string" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                Ok(japl_stdlib::int_to_string(val))
            }

            "float_to_string" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                match val {
                    Value::Float(f) => Ok(Value::String(Arc::from(f.to_string().as_str()))),
                    _ => Ok(Value::String(Arc::from(""))),
                }
            }

            "string_length" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                Ok(japl_stdlib::string_length(val))
            }

            "string_concat" => {
                if args.len() != 2 {
                    return Err(InterpreterError::ArityMismatch {
                        expected: 2,
                        got: args.len(),
                    });
                }
                Ok(japl_stdlib::string_concat(&args[0], &args[1]))
            }

            "list_length" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                Ok(japl_stdlib::list_length(val))
            }

            "list_head" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                match val {
                    Value::List(elems) => {
                        if let Some(first) = elems.first() {
                            Ok(first.clone())
                        } else {
                            Err(InterpreterError::Runtime("head of empty list".to_string()))
                        }
                    }
                    _ => Err(InterpreterError::TypeError(
                        "list_head on non-list".to_string(),
                    )),
                }
            }

            "list_tail" => {
                let val = args
                    .first()
                    .ok_or_else(|| InterpreterError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    })?;
                match val {
                    Value::List(elems) => {
                        if elems.is_empty() {
                            Err(InterpreterError::Runtime("tail of empty list".to_string()))
                        } else {
                            Ok(Value::List(elems[1..].to_vec()))
                        }
                    }
                    _ => Err(InterpreterError::TypeError(
                        "list_tail on non-list".to_string(),
                    )),
                }
            }

            "list_append" => {
                if args.len() != 2 {
                    return Err(InterpreterError::ArityMismatch {
                        expected: 2,
                        got: args.len(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::List(a), Value::List(b)) => {
                        let mut result = a.clone();
                        result.extend(b.iter().cloned());
                        Ok(Value::List(result))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "list_append on non-lists".to_string(),
                    )),
                }
            }

            "__record_update" => {
                if args.len() != 2 {
                    return Err(InterpreterError::ArityMismatch {
                        expected: 2,
                        got: args.len(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::Record(base), Value::Record(updates)) => {
                        let mut result = base.clone();
                        for (name, val) in updates {
                            if let Some(entry) = result.iter_mut().find(|(n, _)| n == name) {
                                entry.1 = val.clone();
                            } else {
                                result.push((name.clone(), val.clone()));
                            }
                        }
                        Ok(Value::Record(result))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "record update on non-records".to_string(),
                    )),
                }
            }

            _ => Err(InterpreterError::UndefinedVariable(format!(
                "unknown builtin: {}",
                name
            ))),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Match a value against a pattern, collecting bindings.
/// Returns true if the pattern matches.
fn match_pattern(
    pattern: &IrPattern,
    value: &Value,
    bindings: &mut HashMap<String, Value>,
) -> bool {
    match pattern {
        IrPattern::Wildcard => true,

        IrPattern::Var(name) => {
            bindings.insert(name.clone(), value.clone());
            true
        }

        IrPattern::Literal(lit) => value == lit,

        IrPattern::Constructor(name, sub_patterns) => {
            if let Value::Constructor(ctor_name, fields) = value {
                if ctor_name.as_ref() == name.as_str() && fields.len() == sub_patterns.len() {
                    for (pat, val) in sub_patterns.iter().zip(fields.iter()) {
                        if !match_pattern(pat, val, bindings) {
                            return false;
                        }
                    }
                    return true;
                }
            }
            false
        }

        IrPattern::Tuple(sub_patterns) => {
            if let Value::Tuple(elems) = value {
                if elems.len() == sub_patterns.len() {
                    for (pat, val) in sub_patterns.iter().zip(elems.iter()) {
                        if !match_pattern(pat, val, bindings) {
                            return false;
                        }
                    }
                    return true;
                }
            }
            false
        }

        IrPattern::Record(field_patterns) => {
            if let Value::Record(fields) = value {
                for (pat_name, pat) in field_patterns {
                    let field_val = fields
                        .iter()
                        .find(|(n, _)| n.as_ref() == pat_name.as_str())
                        .map(|(_, v)| v);
                    match field_val {
                        Some(v) => {
                            if !match_pattern(pat, v, bindings) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            } else {
                false
            }
        }

        IrPattern::List(elem_patterns, rest_pattern) => {
            if let Value::List(elems) = value {
                if let Some(rest_pat) = rest_pattern {
                    // Pattern with rest: [x, y, ..rest]
                    if elems.len() < elem_patterns.len() {
                        return false;
                    }
                    for (pat, val) in elem_patterns.iter().zip(elems.iter()) {
                        if !match_pattern(pat, val, bindings) {
                            return false;
                        }
                    }
                    let rest = Value::List(elems[elem_patterns.len()..].to_vec());
                    match_pattern(rest_pat, &rest, bindings)
                } else {
                    // Exact-length pattern: [x, y, z]
                    if elems.len() != elem_patterns.len() {
                        return false;
                    }
                    for (pat, val) in elem_patterns.iter().zip(elems.iter()) {
                        if !match_pattern(pat, val, bindings) {
                            return false;
                        }
                    }
                    true
                }
            } else {
                false
            }
        }

        IrPattern::Or(patterns) => {
            for pat in patterns {
                let mut local_bindings = HashMap::new();
                if match_pattern(pat, value, &mut local_bindings) {
                    bindings.extend(local_bindings);
                    return true;
                }
            }
            false
        }
    }
}

/// Compute the result of a binary operation on two values.
fn eval_binop(op: IrBinOp, lhs: &Value, rhs: &Value) -> Result<Value, InterpreterError> {
    match (op, lhs, rhs) {
        // Integer arithmetic
        (IrBinOp::Add, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (IrBinOp::Sub, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
        (IrBinOp::Mul, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
        (IrBinOp::Div, Value::Int(_), Value::Int(0)) => Err(InterpreterError::DivisionByZero),
        (IrBinOp::Div, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
        (IrBinOp::Mod, Value::Int(_), Value::Int(0)) => Err(InterpreterError::DivisionByZero),
        (IrBinOp::Mod, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),

        // Float arithmetic
        (IrBinOp::Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (IrBinOp::Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (IrBinOp::Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (IrBinOp::Div, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
        (IrBinOp::Mod, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),

        // Mixed int/float arithmetic
        (IrBinOp::Add, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
        (IrBinOp::Add, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
        (IrBinOp::Sub, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
        (IrBinOp::Sub, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
        (IrBinOp::Mul, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
        (IrBinOp::Mul, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
        (IrBinOp::Div, Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
        (IrBinOp::Div, Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),

        // Integer comparison
        (IrBinOp::Eq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
        (IrBinOp::Neq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a != b)),
        (IrBinOp::Lt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
        (IrBinOp::Gt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
        (IrBinOp::LtEq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (IrBinOp::GtEq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),

        // Float comparison
        (IrBinOp::Eq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a == b)),
        (IrBinOp::Neq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a != b)),
        (IrBinOp::Lt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
        (IrBinOp::Gt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
        (IrBinOp::LtEq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
        (IrBinOp::GtEq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),

        // String comparison
        (IrBinOp::Eq, Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
        (IrBinOp::Neq, Value::String(a), Value::String(b)) => Ok(Value::Bool(a != b)),

        // Bool comparison
        (IrBinOp::Eq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
        (IrBinOp::Neq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a != b)),

        // General equality (constructors, etc.)
        (IrBinOp::Eq, a, b) => Ok(Value::Bool(a == b)),
        (IrBinOp::Neq, a, b) => Ok(Value::Bool(a != b)),

        // Boolean logic
        (IrBinOp::And, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
        (IrBinOp::Or, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

        _ => Err(InterpreterError::TypeError(format!(
            "cannot apply {:?} to {:?} and {:?}",
            op, lhs, rhs
        ))),
    }
}

/// Compute the result of a unary operation on a value.
fn eval_unaryop(op: IrUnaryOp, val: &Value) -> Result<Value, InterpreterError> {
    match (op, val) {
        (IrUnaryOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
        (IrUnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
        (IrUnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
        _ => Err(InterpreterError::TypeError(format!(
            "cannot apply {:?} to {:?}",
            op, val
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use japl_common::FileId;
    use japl_ir::lower::lower;
    use japl_parser::parse;

    fn run_program(source: &str) -> Result<(Value, Vec<String>), String> {
        let (ast, diags) = parse(source, FileId(0));
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == japl_common::Severity::Error)
            .collect();
        if !errors.is_empty() {
            return Err(format!("Parse errors: {:?}", errors));
        }
        let program = lower(&ast).map_err(|e| format!("Lower error: {}", e))?;
        let mut interp = Interpreter::new_capturing();
        let result = interp
            .run_program(&program)
            .map_err(|e| format!("Runtime error: {}", e))?;
        Ok((result, interp.get_output().to_vec()))
    }

    #[test]
    fn test_simple_addition() {
        let (val, _) = run_program("fn main() -> Int = 1 + 2").unwrap();
        assert_eq!(val, Value::Int(3));
    }

    #[test]
    fn test_let_binding() {
        let (val, _) = run_program(
            "fn main() -> Int =\n  let x = 10\n  let y = 20\n  x + y",
        )
        .unwrap();
        assert_eq!(val, Value::Int(30));
    }

    #[test]
    fn test_function_call() {
        let (val, _) = run_program(
            "fn double(n: Int) -> Int = n * 2\n\nfn main() -> Int = double(21)",
        )
        .unwrap();
        assert_eq!(val, Value::Int(42));
    }

    #[test]
    fn test_recursion() {
        let source = r#"
fn fact(n: Int) -> Int =
  if n <= 1 then 1 else n * fact(n - 1)

fn main() -> Int = fact(5)
"#;
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(120));
    }

    #[test]
    fn test_if_else() {
        let (val, _) = run_program(
            "fn main() -> Int = if True then 42 else 0",
        )
        .unwrap();
        assert_eq!(val, Value::Int(42));
    }

    #[test]
    fn test_string_concat() {
        let source = r#"fn main() -> String = "hello" ++ " " ++ "world""#;
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::String(Arc::from("hello world")));
    }

    #[test]
    fn test_println_output() {
        let source = r#"
fn main() =
  println("hello")
"#;
        let (_, output) = run_program(source).unwrap();
        assert_eq!(output, vec!["hello"]);
    }

    #[test]
    fn test_list_literal() {
        let (val, _) = run_program("fn main() = [1, 2, 3]").unwrap();
        assert_eq!(
            val,
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_record_literal() {
        let source = r#"fn main() = { x = 1, y = 2 }"#;
        let (val, _) = run_program(source).unwrap();
        assert_eq!(
            val,
            Value::Record(vec![
                (Arc::from("x"), Value::Int(1)),
                (Arc::from("y"), Value::Int(2)),
            ])
        );
    }

    #[test]
    fn test_field_access() {
        let source = r#"
fn main() -> Int =
  let r = { x = 42, y = 10 }
  r.x
"#;
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(42));
    }

    #[test]
    fn test_pattern_match_int() {
        let source = "fn describe(n: Int) -> String = match n with\n  | 0 -> \"zero\"\n  | 1 -> \"one\"\n  | _ -> \"other\"\n\nfn main() -> String = describe(1)\n";
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::String(Arc::from("one")));
    }

    #[test]
    fn test_pattern_match_constructor() {
        let source = "type Option[a] =\n  | Some(a)\n  | None\n\nfn unwrap_or(opt: Option[Int], default: Int) -> Int = match opt with\n  | Some(x) -> x\n  | None -> default\n\nfn main() -> Int =\n  let a = Some(42)\n  unwrap_or(a, 0)\n";
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(42));
    }

    #[test]
    fn test_constructor_none() {
        let source = "type Option[a] =\n  | Some(a)\n  | None\n\nfn unwrap_or(opt: Option[Int], default: Int) -> Int = match opt with\n  | Some(x) -> x\n  | None -> default\n\nfn main() -> Int = unwrap_or(None, 99)\n";
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(99));
    }

    #[test]
    fn test_boolean_ops() {
        let (val, _) = run_program("fn main() -> Bool = True && False || True").unwrap();
        assert_eq!(val, Value::Bool(true));
    }

    #[test]
    fn test_comparison() {
        let (val, _) = run_program("fn main() -> Bool = 10 > 5").unwrap();
        assert_eq!(val, Value::Bool(true));
    }

    #[test]
    fn test_negation() {
        let (val, _) = run_program("fn main() -> Int = 0 - 42").unwrap();
        assert_eq!(val, Value::Int(-42));
    }

    #[test]
    fn test_int_to_string() {
        let (val, _) = run_program("fn main() -> String = int_to_string(42)").unwrap();
        assert_eq!(val, Value::String(Arc::from("42")));
    }

    #[test]
    fn test_fibonacci() {
        let source = "fn fib(n: Int) -> Int = match n with\n  | 0 -> 0\n  | 1 -> 1\n  | n -> fib(n - 1) + fib(n - 2)\n\nfn main() -> Int = fib(10)\n";
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(55));
    }

    #[test]
    fn test_show_function() {
        let (val, _) = run_program(r#"fn main() -> String = show(42)"#).unwrap();
        assert_eq!(val, Value::String(Arc::from("42")));
    }

    #[test]
    fn test_multiple_functions() {
        let source = r#"
fn add(a: Int, b: Int) -> Int = a + b
fn mul(a: Int, b: Int) -> Int = a * b
fn main() -> Int = mul(add(2, 3), add(4, 6))
"#;
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(50));
    }

    #[test]
    fn test_division_by_zero() {
        let result = run_program("fn main() -> Int = 10 / 0");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_main() {
        let result = run_program("fn foo() -> Int = 42");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("main"));
    }

    #[test]
    fn test_tuple() {
        let (val, _) = run_program("fn main() = (1, 2, 3)").unwrap();
        assert_eq!(
            val,
            Value::Tuple(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_nested_match() {
        let source = "type Expr =\n  | Num(Int)\n  | Add(Int, Int)\n\nfn run_expr(e: Expr) -> Int = match e with\n  | Num(n) -> n\n  | Add(a, b) -> a + b\n\nfn main() -> Int = run_expr(Add(10, 32))\n";
        let (val, _) = run_program(source).unwrap();
        assert_eq!(val, Value::Int(42));
    }
}

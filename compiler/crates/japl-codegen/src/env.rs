//! Environment (scope) management for the interpreter.
//!
//! The environment is a stack of scopes. Each scope is a map from variable
//! names to values. Looking up a variable walks the stack from top to bottom.

use std::collections::HashMap;

use japl_runtime::Value;

/// A lexical environment with nested scopes.
#[derive(Debug, Clone)]
pub struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    /// Create a new environment with a single empty scope.
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope onto the stack.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the topmost scope.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Bind a variable in the current (topmost) scope.
    pub fn bind(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Look up a variable, searching from the innermost scope outward.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }

    /// Create a snapshot of the current environment (for closures).
    pub fn snapshot(&self) -> Environment {
        self.clone()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_and_lookup() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(42));
        assert_eq!(env.lookup("x"), Some(&Value::Int(42)));
        assert_eq!(env.lookup("y"), None);
    }

    #[test]
    fn test_nested_scopes() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(1));
        env.push_scope();
        env.bind("x".to_string(), Value::Int(2));
        assert_eq!(env.lookup("x"), Some(&Value::Int(2)));
        env.pop_scope();
        assert_eq!(env.lookup("x"), Some(&Value::Int(1)));
    }

    #[test]
    fn test_outer_scope_visible() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(1));
        env.push_scope();
        env.bind("y".to_string(), Value::Int(2));
        assert_eq!(env.lookup("x"), Some(&Value::Int(1)));
        assert_eq!(env.lookup("y"), Some(&Value::Int(2)));
    }

    #[test]
    fn test_snapshot() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(42));
        let snap = env.snapshot();
        env.bind("x".to_string(), Value::Int(99));
        assert_eq!(snap.lookup("x"), Some(&Value::Int(42)));
        assert_eq!(env.lookup("x"), Some(&Value::Int(99)));
    }
}

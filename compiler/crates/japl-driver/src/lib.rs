//! japl-driver: Compiler driver library for JAPL.
//!
//! Provides the pipeline functions used by the CLI binary.
//! The main entry point is in main.rs.

use japl_codegen::Interpreter;
use japl_common::FileId;
use japl_ir::lower::lower;
use japl_parser::parse;

/// Run a JAPL source string through the full pipeline: parse, lower, interpret.
/// Returns the output lines (if capturing) or Ok(()) for normal execution.
pub fn run_source(source: &str) -> Result<(), String> {
    let file_id = FileId(0);
    let (ast, diags) = parse(source, file_id);

    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == japl_common::Severity::Error)
        .collect();
    if !errors.is_empty() {
        return Err(format!("Parse errors: {:?}", errors));
    }

    let program = lower(&ast).map_err(|e| format!("Lower error: {}", e))?;
    let mut interpreter = Interpreter::new();
    interpreter
        .run_program(&program)
        .map_err(|e| format!("Runtime error: {}", e))?;
    Ok(())
}

/// Run a JAPL source string and capture output (for testing).
pub fn run_source_capturing(source: &str) -> Result<Vec<String>, String> {
    let file_id = FileId(0);
    let (ast, diags) = parse(source, file_id);

    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == japl_common::Severity::Error)
        .collect();
    if !errors.is_empty() {
        return Err(format!("Parse errors: {:?}", errors));
    }

    let program = lower(&ast).map_err(|e| format!("Lower error: {}", e))?;
    let mut interpreter = Interpreter::new_capturing();
    interpreter
        .run_program(&program)
        .map_err(|e| format!("Runtime error: {}", e))?;
    Ok(interpreter.get_output().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let output = run_source_capturing("fn main() =\n  println(\"Hello from JAPL!\")").unwrap();
        assert_eq!(output, vec!["Hello from JAPL!"]);
    }

    #[test]
    fn test_fibonacci_program() {
        let source = "fn fib(n: Int) -> Int = match n with\n  | 0 -> 0\n  | 1 -> 1\n  | n -> fib(n - 1) + fib(n - 2)\n\nfn main() =\n  println(int_to_string(fib(10)))\n";
        let output = run_source_capturing(source).unwrap();
        assert_eq!(output, vec!["55"]);
    }

    #[test]
    fn test_pattern_match_program() {
        let source = "type Shape =\n  | Circle(Float)\n  | Rectangle(Float, Float)\n\nfn area(shape: Shape) -> Float = match shape with\n  | Circle(r) -> 3.14159 * r * r\n  | Rectangle(w, h) -> w * h\n\nfn main() =\n  let c = Circle(5.0)\n  let r = Rectangle(3.0, 4.0)\n  println(\"Circle area: \" ++ show(area(c)))\n  println(\"Rectangle area: \" ++ show(area(r)))\n";
        let output = run_source_capturing(source).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], "Circle area: 78.53975");
        assert_eq!(output[1], "Rectangle area: 12");
    }

    #[test]
    fn test_no_main_error() {
        let result = run_source("fn foo() -> Int = 42");
        assert!(result.is_err());
    }
}

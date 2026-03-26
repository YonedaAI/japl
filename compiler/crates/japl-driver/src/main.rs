//! japlc: The JAPL compiler driver.
//!
//! Reads .japl source files, lexes, parses, lowers to IR, and interprets.

use clap::Parser as ClapParser;
use japl_ast::pretty;
use japl_codegen::Interpreter;
use japl_common::FileId;
use japl_ir::lower::lower;
use japl_lexer::lex_all;
use japl_parser::parse;
use std::path::PathBuf;
use std::process;

#[derive(ClapParser, Debug)]
#[command(name = "japlc", version, about = "The JAPL compiler")]
struct Cli {
    /// Subcommand (optional): use `run` to execute a file
    #[command(subcommand)]
    command: Option<Command>,

    /// Source file to compile (when not using a subcommand)
    #[arg(global = true)]
    input: Option<PathBuf>,

    /// Print the token stream instead of the AST
    #[arg(long, global = true)]
    tokens: bool,

    /// Print the AST
    #[arg(long, global = true)]
    ast: bool,

    /// Type-check only (do not interpret)
    #[arg(long, global = true)]
    check: bool,

    /// Print the IR
    #[arg(long, global = true)]
    ir: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Run a JAPL program: lex, parse, lower, interpret
    Run {
        /// Source file to run
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    // Determine the input file
    let input = match (&cli.command, &cli.input) {
        (Some(Command::Run { file }), _) => file.clone(),
        (None, Some(input)) => input.clone(),
        (None, None) => {
            eprintln!("error: no input file specified");
            eprintln!("Usage: japlc <file.japl> or japlc run <file.japl>");
            process::exit(1);
        }
    };

    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input.display(), e);
            process::exit(1);
        }
    };

    let file_id = FileId(0);

    // Token dump mode
    if cli.tokens {
        let (tokens, lex_diags) = lex_all(&source, file_id);
        for tok in &tokens {
            println!(
                "{:4}..{:4}  {:?}  {:?}",
                tok.span.start, tok.span.end, tok.token, tok.text
            );
        }
        if !lex_diags.is_empty() {
            eprintln!("\n--- Lexer diagnostics ---");
            for d in &lex_diags {
                eprintln!("{}", d);
            }
        }
        return;
    }

    // Parse
    let (ast, parse_diags) = parse(&source, file_id);

    // Report parse diagnostics
    let has_errors = report_diagnostics(&parse_diags, &source, &input);
    if has_errors {
        eprintln!("\nCompilation failed with errors.");
        process::exit(1);
    }

    // AST dump mode
    if cli.ast {
        let pretty_str = pretty::pretty_print(&ast);
        println!("{}", pretty_str);
        return;
    }

    // Check-only mode (placeholder -- type checker not wired in yet)
    if cli.check {
        println!("Parse successful. Type checking not yet connected.");
        return;
    }

    // Lower to IR
    let program = match lower(&ast) {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("error: IR lowering failed: {}", e);
            process::exit(1);
        }
    };

    // IR dump mode
    if cli.ir {
        println!("--- IR Program ---");
        println!("Functions: {}", program.functions.len());
        for f in &program.functions {
            println!("  fn {}({}) = {:?}", f.name, f.params.join(", "), f.body);
        }
        println!("Type definitions: {}", program.type_defs.len());
        for td in &program.type_defs {
            println!("  type {} =", td.name);
            for v in &td.variants {
                println!("    | {}({})", v.name, v.arity);
            }
        }
        return;
    }

    // Interpret
    let mut interpreter = Interpreter::new();
    match interpreter.run_program(&program) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("error: runtime error: {}", e);
            process::exit(1);
        }
    }
}

/// Report diagnostics to stderr. Returns true if there are errors.
fn report_diagnostics(
    diags: &[japl_common::Diagnostic],
    source: &str,
    input: &PathBuf,
) -> bool {
    let has_errors = diags
        .iter()
        .any(|d| d.severity == japl_common::Severity::Error);

    if !diags.is_empty() {
        for d in diags {
            let severity = match d.severity {
                japl_common::Severity::Error => "error",
                japl_common::Severity::Warning => "warning",
                japl_common::Severity::Info => "info",
                japl_common::Severity::Hint => "hint",
            };
            eprint!("{}: {}", severity, d.message);
            for label in &d.labels {
                let (line, col) = byte_offset_to_line_col(source, label.span.start);
                eprint!(" [{}:{}:{}]", input.display(), line, col);
            }
            eprintln!();
            for note in &d.notes {
                eprintln!("  note: {}", note);
            }
        }
    }

    has_errors
}

fn byte_offset_to_line_col(source: &str, offset: u32) -> (usize, usize) {
    let offset = offset as usize;
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

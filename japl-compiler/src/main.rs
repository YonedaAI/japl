mod token;
mod lexer;
mod ast;
mod parser;
mod ir;
mod lower;
mod emit_wat;
mod error;
mod checker;
mod types;
mod formatter;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn parse_file(path: &Path) -> Result<ast::Program, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Error reading {}: {}", path.display(), e))?;
    let file_str = path.display().to_string();

    let mut lex = lexer::Lexer::new(&source, &file_str);
    let tokens = lex.tokenize().map_err(|e| format!("{}", e))?;

    let mut par = parser::Parser::new(tokens, &file_str);
    par.parse_program().map_err(|e| format!("{}", e))
}

/// Resolve imports: parse imported files and merge their pub fn defs into the main program.
fn resolve_imports(program: &mut ast::Program, base_dir: &Path, visited: &mut HashSet<String>) {
    let imports: Vec<ast::ImportDef> = program.items.iter().filter_map(|item| {
        if let ast::TopLevel::Import(imp) = item {
            Some(imp.clone())
        } else {
            None
        }
    }).collect();

    for imp in imports {
        let module_name = imp.module_path.join("/");
        if visited.contains(&module_name) {
            continue;
        }
        visited.insert(module_name.clone());

        // Try base_dir/Module.japl
        let module_file = base_dir.join(format!("{}.japl", imp.module_path.join("/")));
        if let Ok(mut mod_program) = parse_file(&module_file) {
            // Recursively resolve imports in the module
            resolve_imports(&mut mod_program, base_dir, visited);

            let import_names: HashSet<String> = imp.names.into_iter().collect();

            // Add module's items (type defs, and imported fn defs) to main program
            for item in mod_program.items {
                match &item {
                    ast::TopLevel::FnDef(fd) => {
                        if import_names.is_empty() || import_names.contains(&fd.name) {
                            program.items.push(item.clone());
                        }
                    }
                    ast::TopLevel::TypeDef(_) => {
                        program.items.push(item.clone());
                    }
                    ast::TopLevel::Const(_) => {
                        program.items.push(item.clone());
                    }
                    _ => {}
                }
            }
        } else {
            eprintln!("Warning: could not resolve import {}", module_name);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: japl-compiler <build|check|fmt> <file.japl> [--emit-wat] [--out <dir>] [--strict]");
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "fmt" => {
            if args.len() < 3 {
                eprintln!("Usage: japl-compiler fmt <file.japl>");
                std::process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let source = match std::fs::read_to_string(&input_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", input_path.display(), e);
                    std::process::exit(1);
                }
            };
            let file_str = input_path.display().to_string();
            let mut lex = lexer::Lexer::new(&source, &file_str);
            let tokens = match lex.tokenize() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let mut par = parser::Parser::new(tokens, &file_str);
            let program = match par.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let formatted = formatter::format_program(&program);
            print!("{}", formatted);
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("Usage: japl-compiler check <file.japl> [--strict]");
                std::process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let strict = args.iter().any(|a| a == "--strict");

            let source = match std::fs::read_to_string(&input_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", input_path.display(), e);
                    std::process::exit(1);
                }
            };
            let file_str = input_path.display().to_string();
            let mut lex = lexer::Lexer::new(&source, &file_str);
            let tokens = match lex.tokenize() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let mut par = parser::Parser::new(tokens, &file_str);
            let program = match par.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            let errors = checker::check_program(&program, strict);
            if errors.is_empty() {
                eprintln!("No errors found.");
            } else {
                for err in &errors {
                    eprintln!("{}", err);
                }
                std::process::exit(1);
            }
        }
        "build" => {
            if args.len() < 3 {
                eprintln!("Usage: japl-compiler build <file.japl> [--emit-wat] [--out <dir>]");
                std::process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let mut emit_wat_flag = false;
            let mut out_dir = PathBuf::from("build");
            let strict = args.iter().any(|a| a == "--strict");

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--emit-wat" => emit_wat_flag = true,
                    "--out" => {
                        i += 1;
                        if i < args.len() {
                            out_dir = PathBuf::from(&args[i]);
                        }
                    }
                    "--strict" => {} // already handled
                    _ => {
                        eprintln!("Unknown flag: {}", args[i]);
                        std::process::exit(1);
                    }
                }
                i += 1;
            }

            let mut program = match parse_file(&input_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            // Resolve imports
            let base_dir = input_path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let mut visited = HashSet::new();
            resolve_imports(&mut program, &base_dir, &mut visited);

            // Type check (non-fatal unless --strict for effect checking)
            let errors = checker::check_program(&program, strict);
            if !errors.is_empty() {
                for err in &errors {
                    eprintln!("{}", err);
                }
                // Type errors are fatal
                if errors.iter().any(|e| e.contains("type error")) {
                    std::process::exit(1);
                }
                // Effect errors only fatal in --strict mode
                if strict && errors.iter().any(|e| e.contains("effect")) {
                    std::process::exit(1);
                }
            }

            let file_name = input_path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Lower
            let mut lowerer = lower::Lowerer::new();
            let ir_module = lowerer.lower_program(&program);

            // Emit WAT
            let emitter = emit_wat::WatEmitter::new(ir_module);
            let wat = emitter.emit();

            // Create output directory
            std::fs::create_dir_all(&out_dir).ok();

            // Write .wat file
            let wat_path = out_dir.join(format!("{}.wat", file_name));
            if let Err(e) = std::fs::write(&wat_path, &wat) {
                eprintln!("Error writing WAT: {}", e);
                std::process::exit(1);
            }

            // Run wat2wasm
            let wasm_path = out_dir.join(format!("{}.wasm", file_name));
            let result = Command::new("wat2wasm")
                .arg(&wat_path)
                .arg("-o")
                .arg(&wasm_path)
                .output();

            match result {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!("wat2wasm failed:");
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run wat2wasm: {}", e);
                    std::process::exit(1);
                }
            }

            if !emit_wat_flag {
                std::fs::remove_file(&wat_path).ok();
            }

            eprintln!("Compiled {} -> {}", input_path.display(), wasm_path.display());
        }
        _ => {
            eprintln!("Unknown command: {}. Use build, check, or fmt.", command);
            std::process::exit(1);
        }
    }
}

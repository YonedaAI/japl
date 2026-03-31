pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod ir;
pub mod lower;
pub mod emit_wat;
pub mod error;
pub mod checker;
pub mod types;
pub mod formatter;

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

/// Compile a .japl file to .wasm, returning the path to the .wasm file.
/// If `target` is "component", emit Component Model canonical ABI imports.
pub fn compile(path: &str, out_dir: &str) -> Result<String, String> {
    compile_with_target(path, out_dir, "local")
}

pub fn compile_with_target(path: &str, out_dir: &str, target: &str) -> Result<String, String> {
    let input_path = PathBuf::from(path);

    let mut program = parse_file(&input_path)?;

    // Resolve imports
    let base_dir = input_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut visited = HashSet::new();
    resolve_imports(&mut program, &base_dir, &mut visited);

    // Type check (non-fatal unless strict for effect checking)
    let errors = checker::check_program(&program, false);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("{}", err);
        }
        // Type errors are fatal
        if errors.iter().any(|e| e.contains("type error")) {
            return Err(errors.join("\n"));
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
    let component_target = target == "component";
    let emitter = emit_wat::WatEmitter::new(ir_module, component_target);
    let wat = emitter.emit();

    // Create output directory
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Error creating output directory: {}", e))?;

    let out_path = PathBuf::from(out_dir);

    // Write .wat file
    let wat_path = out_path.join(format!("{}.wat", file_name));
    std::fs::write(&wat_path, &wat)
        .map_err(|e| format!("Error writing WAT: {}", e))?;

    // Run wat2wasm
    let wasm_path = out_path.join(format!("{}.wasm", file_name));
    let result = Command::new("wat2wasm")
        .arg(&wat_path)
        .arg("-o")
        .arg(&wasm_path)
        .output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("wat2wasm failed:\n{}", stderr));
            }
        }
        Err(e) => {
            return Err(format!("Failed to run wat2wasm: {}", e));
        }
    }

    // Remove .wat file (keep for debugging when JAPL_KEEP_WAT is set)
    if std::env::var("JAPL_KEEP_WAT").is_err() {
        std::fs::remove_file(&wat_path).ok();
    }

    eprintln!("Compiled {} -> {}", input_path.display(), wasm_path.display());
    Ok(wasm_path.display().to_string())
}

/// Type-check a .japl file.
pub fn check(path: &str) -> Result<(), String> {
    let input_path = PathBuf::from(path);

    let source = std::fs::read_to_string(&input_path)
        .map_err(|e| format!("Error reading {}: {}", input_path.display(), e))?;
    let file_str = input_path.display().to_string();

    let mut lex = lexer::Lexer::new(&source, &file_str);
    let tokens = lex.tokenize().map_err(|e| format!("{}", e))?;

    let mut par = parser::Parser::new(tokens, &file_str);
    let program = par.parse_program().map_err(|e| format!("{}", e))?;

    let errors = checker::check_program(&program, false);
    if errors.is_empty() {
        eprintln!("No errors found.");
        Ok(())
    } else {
        for err in &errors {
            eprintln!("{}", err);
        }
        Err(errors.join("\n"))
    }
}

/// Format a .japl file and print to stdout.
pub fn format(path: &str) -> Result<(), String> {
    let input_path = PathBuf::from(path);

    let source = std::fs::read_to_string(&input_path)
        .map_err(|e| format!("Error reading {}: {}", input_path.display(), e))?;
    let file_str = input_path.display().to_string();

    let mut lex = lexer::Lexer::new(&source, &file_str);
    let tokens = lex.tokenize().map_err(|e| format!("{}", e))?;

    let mut par = parser::Parser::new(tokens, &file_str);
    let program = par.parse_program().map_err(|e| format!("{}", e))?;

    let formatted = formatter::format_program(&program);
    print!("{}", formatted);
    Ok(())
}

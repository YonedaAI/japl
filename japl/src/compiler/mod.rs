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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for stdlib search paths.
#[derive(Clone, Debug)]
pub struct CompilerConfig {
    /// Ordered list of directories to search for imports.
    /// Default: relative to source file, then stdlib/.
    pub search_paths: Vec<PathBuf>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        CompilerConfig {
            search_paths: Vec::new(),
        }
    }
}

fn parse_file(path: &Path) -> Result<ast::Program, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Error reading {}: {}", path.display(), e))?;
    let file_str = path.display().to_string();

    let mut lex = lexer::Lexer::new(&source, &file_str);
    let tokens = lex.tokenize().map_err(|e| format!("{}", e))?;

    let mut par = parser::Parser::new(tokens, &file_str);
    par.parse_program().map_err(|e| format!("{}", e))
}

/// Find a module file by searching: (a) base_dir, (b) each search_path in order.
fn find_module_file(module_name: &str, base_dir: &Path, search_paths: &[PathBuf]) -> Option<PathBuf> {
    let file_name = format!("{}.japl", module_name);

    // (a) Relative to source file
    let candidate = base_dir.join(&file_name);
    if candidate.exists() {
        return Some(candidate);
    }

    // (b) Each configured search path
    for sp in search_paths {
        let candidate = sp.join(&file_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Track which modules are currently being resolved (for circular dependency detection).
/// `visited` = fully resolved modules, `in_progress` = currently on the resolution stack.
fn resolve_imports(
    program: &mut ast::Program,
    base_dir: &Path,
    search_paths: &[PathBuf],
    visited: &mut HashSet<String>,
    in_progress: &mut Vec<String>,
    module_exports: &mut HashMap<String, HashSet<String>>,
) -> Result<(), String> {
    let imports: Vec<ast::ImportDef> = program.items.iter().filter_map(|item| {
        if let ast::TopLevel::Import(imp) = item {
            Some(imp.clone())
        } else {
            None
        }
    }).collect();

    for imp in imports {
        let module_name = imp.module_path.join("/");

        // Circular dependency detection
        if in_progress.contains(&module_name) {
            let cycle = in_progress.iter()
                .skip_while(|m| **m != module_name)
                .cloned()
                .collect::<Vec<_>>();
            return Err(format!(
                "circular import detected: {} -> {}",
                cycle.join(" -> "),
                module_name
            ));
        }

        if visited.contains(&module_name) {
            // Already resolved on a previous pass; items were added then.
            continue;
        }

        // Try to find the module file
        let module_file = match find_module_file(&module_name, base_dir, search_paths) {
            Some(p) => p,
            None => {
                return Err(format!("error: could not resolve import '{}'", module_name));
            }
        };

        let mut mod_program = parse_file(&module_file)?;

        // Push onto in-progress stack for cycle detection
        in_progress.push(module_name.clone());

        // Recursively resolve imports in the module, using the module's own directory as base
        let mod_base = module_file.parent().unwrap_or(Path::new(".")).to_path_buf();
        resolve_imports(&mut mod_program, &mod_base, search_paths, visited, in_progress, module_exports)?;

        // Pop from in-progress stack
        in_progress.pop();
        visited.insert(module_name.clone());

        let import_names: HashSet<String> = imp.names.iter().cloned().collect();

        // Collect the pub exports of this module
        let mut pub_names = HashSet::new();
        for item in &mod_program.items {
            match item {
                ast::TopLevel::FnDef(fd) if fd.is_pub => {
                    pub_names.insert(fd.name.clone());
                }
                _ => {}
            }
        }
        module_exports.insert(module_name.clone(), pub_names.clone());

        // Add module's pub items (and type defs) to the importing program
        for item in mod_program.items {
            match &item {
                ast::TopLevel::FnDef(fd) => {
                    // Only export pub functions
                    if fd.is_pub {
                        if import_names.is_empty() || import_names.contains(&fd.name) {
                            // Also store a qualified name: Module__function
                            let qualified_name = format!("{}_{}", imp.module_path.join("_"), fd.name);
                            // Push the original item
                            program.items.push(item.clone());
                            // Push a qualified-name alias
                            let mut aliased = fd.clone();
                            aliased.name = qualified_name;
                            program.items.push(ast::TopLevel::FnDef(aliased));
                        }
                    }
                    // Non-pub functions are NOT exported
                }
                ast::TopLevel::TypeDef(_) => {
                    program.items.push(item.clone());
                }
                ast::TopLevel::Const(_) => {
                    program.items.push(item.clone());
                }
                ast::TopLevel::ForeignFn(_) => {
                    // Foreign declarations must be carried over so that
                    // pub wrapper functions in the imported module can
                    // resolve their FFI calls.
                    program.items.push(item.clone());
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Compute the default stdlib path: look relative to the executable, then fall back to cwd.
fn default_stdlib_path() -> Option<PathBuf> {
    // Try: executable_dir/../stdlib/
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // For development: exe is in japl/target/debug or japl/target/release
            // stdlib is at repo_root/stdlib/
            let candidate = exe_dir.join("../../stdlib");
            if candidate.exists() {
                return Some(candidate.canonicalize().unwrap_or(candidate));
            }
            // Also try: exe_dir/../stdlib (if installed)
            let candidate = exe_dir.join("../stdlib");
            if candidate.exists() {
                return Some(candidate.canonicalize().unwrap_or(candidate));
            }
        }
    }
    // Fall back: cwd/stdlib/
    let cwd_stdlib = PathBuf::from("stdlib");
    if cwd_stdlib.exists() {
        return Some(cwd_stdlib.canonicalize().unwrap_or(cwd_stdlib));
    }
    None
}

/// Compile a .japl file to .wasm, returning the path to the .wasm file.
pub fn compile(path: &str, out_dir: &str) -> Result<String, String> {
    compile_with_target(path, out_dir, "local")
}

pub fn compile_with_target(path: &str, out_dir: &str, target: &str) -> Result<String, String> {
    compile_full(path, out_dir, target, None)
}

pub fn compile_full(path: &str, out_dir: &str, target: &str, stdlib_path: Option<&str>) -> Result<String, String> {
    let input_path = PathBuf::from(path);

    let mut program = parse_file(&input_path)?;

    // Build search paths
    let base_dir = input_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // Add explicit stdlib path if provided
    if let Some(sp) = stdlib_path {
        let p = PathBuf::from(sp);
        if p.exists() {
            search_paths.push(p);
        }
    } else {
        // Use default stdlib discovery
        if let Some(sp) = default_stdlib_path() {
            search_paths.push(sp);
        }
    }

    // Resolve imports with cycle detection
    let mut visited = HashSet::new();
    let mut in_progress = Vec::new();
    let mut module_exports = HashMap::new();
    resolve_imports(&mut program, &base_dir, &search_paths, &mut visited, &mut in_progress, &mut module_exports)?;

    // Rewrite qualified calls: Module.func(...) -> Module_func(...)
    rewrite_qualified_calls(&mut program);

    // Type check with effect enforcement enabled by default
    let errors = checker::check_program(&program, true);
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("{}", err);
        }
        // Type errors and effect errors are fatal
        if errors.iter().any(|e| e.contains("type error") || e.contains("effect error")) {
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

/// Rewrite qualified calls like Module.func(args) into Module_func(args).
/// This transforms FieldAccess(Ident("Module"), "func") call expressions.
fn rewrite_qualified_calls(program: &mut ast::Program) {
    // Collect module names (from imports)
    let module_names: HashSet<String> = program.items.iter().filter_map(|item| {
        if let ast::TopLevel::Import(imp) = item {
            Some(imp.module_path.join("_"))
        } else {
            None
        }
    }).collect();

    for item in &mut program.items {
        if let ast::TopLevel::FnDef(fd) = item {
            rewrite_expr_qualified(&mut fd.body, &module_names);
        }
    }
}

fn rewrite_expr_qualified(expr: &mut ast::Expr, module_names: &HashSet<String>) {
    match expr {
        ast::Expr::Call(func, args) => {
            // Check if func is FieldAccess(Ident(module), method)
            if let ast::Expr::FieldAccess(base, method) = func.as_ref() {
                if let ast::Expr::Ident(module) = base.as_ref() {
                    if module_names.contains(module) {
                        let qualified = format!("{}_{}", module, method);
                        *func = Box::new(ast::Expr::Ident(qualified));
                    }
                }
            }
            // Recurse into func and args
            rewrite_expr_qualified(func, module_names);
            for arg in args.iter_mut() {
                rewrite_expr_qualified(arg, module_names);
            }
        }
        ast::Expr::BinOp(_, left, right) => {
            rewrite_expr_qualified(left, module_names);
            rewrite_expr_qualified(right, module_names);
        }
        ast::Expr::If(cond, then, else_) => {
            rewrite_expr_qualified(cond, module_names);
            rewrite_expr_qualified(then, module_names);
            if let Some(e) = else_ {
                rewrite_expr_qualified(e, module_names);
            }
        }
        ast::Expr::Block(stmts, final_expr) => {
            for stmt in stmts.iter_mut() {
                match stmt {
                    ast::Stmt::Let(_, e) | ast::Stmt::LetTyped(_, _, e) | ast::Stmt::Expr(e) => {
                        rewrite_expr_qualified(e, module_names);
                    }
                }
            }
            if let Some(e) = final_expr {
                rewrite_expr_qualified(e, module_names);
            }
        }
        ast::Expr::Lambda(_, _, body) => {
            rewrite_expr_qualified(body, module_names);
        }
        ast::Expr::Match(scrutinee, arms) => {
            rewrite_expr_qualified(scrutinee, module_names);
            for arm in arms.iter_mut() {
                rewrite_expr_qualified(&mut arm.body, module_names);
                if let Some(g) = &mut arm.guard {
                    rewrite_expr_qualified(g, module_names);
                }
            }
        }
        ast::Expr::Pipe(left, right) => {
            rewrite_expr_qualified(left, module_names);
            rewrite_expr_qualified(right, module_names);
        }
        ast::Expr::Record(fields) => {
            for (_, val) in fields.iter_mut() {
                rewrite_expr_qualified(val, module_names);
            }
        }
        ast::Expr::FieldAccess(base, _) => {
            rewrite_expr_qualified(base, module_names);
        }
        ast::Expr::RecordUpdate(base, fields) => {
            rewrite_expr_qualified(base, module_names);
            for (_, val) in fields.iter_mut() {
                rewrite_expr_qualified(val, module_names);
            }
        }
        ast::Expr::Receive(arms) => {
            for arm in arms.iter_mut() {
                rewrite_expr_qualified(&mut arm.body, module_names);
                if let Some(g) = &mut arm.guard {
                    rewrite_expr_qualified(g, module_names);
                }
            }
        }
        ast::Expr::Tuple(exprs) => {
            for e in exprs.iter_mut() {
                rewrite_expr_qualified(e, module_names);
            }
        }
        ast::Expr::TupleAccess(e, _) => {
            rewrite_expr_qualified(e, module_names);
        }
        ast::Expr::UseExpr(_, resource, body) => {
            rewrite_expr_qualified(resource, module_names);
            rewrite_expr_qualified(body, module_names);
        }
        _ => {}
    }
}

/// Type-check a .japl file.
/// When `strict` is true, Pid/Int implicit conversions emit warnings.
pub fn check(path: &str, strict: bool) -> Result<(), String> {
    let input_path = PathBuf::from(path);

    let mut program = parse_file(&input_path)?;

    // Resolve imports so imported functions are available for type checking
    let base_dir = input_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let search_paths: Vec<PathBuf> = default_stdlib_path().into_iter().collect();
    let mut visited = HashSet::new();
    let mut in_progress = Vec::new();
    let mut module_exports = HashMap::new();
    if let Err(e) = resolve_imports(&mut program, &base_dir, &search_paths, &mut visited, &mut in_progress, &mut module_exports) {
        eprintln!("import error: {}", e);
        // Continue type checking even with import errors, but report them
    }

    let errors = checker::check_program(&program, strict);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_module_file_base_dir() {
        // Should find files in the base directory
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../stdlib");
        let result = find_module_file("String", &base, &[]);
        assert!(result.is_some(), "Should find String.japl in stdlib dir");
    }

    #[test]
    fn test_find_module_file_search_path() {
        let base = PathBuf::from("/nonexistent");
        let stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../stdlib");
        let result = find_module_file("String", &base, &[stdlib]);
        assert!(result.is_some(), "Should find String.japl via search path");
    }

    #[test]
    fn test_find_module_file_not_found() {
        let base = PathBuf::from("/nonexistent");
        let result = find_module_file("NoSuchModule", &base, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_circular_import_detection() {
        // Create two temp files that import each other
        let tmp = std::env::temp_dir().join("japl_test_circular");
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("A.japl"), "import B\nfn main() { 0 }").unwrap();
        std::fs::write(tmp.join("B.japl"), "import A\npub fn b() -> Int { 1 }").unwrap();

        let mut program = parse_file(&tmp.join("A.japl")).unwrap();
        let mut visited = HashSet::new();
        let mut in_progress = Vec::new();
        let mut module_exports = HashMap::new();
        let result = resolve_imports(
            &mut program,
            &tmp,
            &[],
            &mut visited,
            &mut in_progress,
            &mut module_exports,
        );
        assert!(result.is_err(), "Should detect circular import");
        let err = result.unwrap_err();
        assert!(err.contains("circular import"), "Error should mention circular import: {}", err);

        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_visibility_enforcement() {
        // Create a module with pub and private functions
        let tmp = std::env::temp_dir().join("japl_test_visibility");
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("Mod.japl"), "pub fn public_fn() -> Int { 1 }\nfn private_fn() -> Int { 2 }").unwrap();
        std::fs::write(tmp.join("main.japl"), "import Mod\nfn main() { public_fn() }").unwrap();

        let mut program = parse_file(&tmp.join("main.japl")).unwrap();
        let mut visited = HashSet::new();
        let mut in_progress = Vec::new();
        let mut module_exports = HashMap::new();
        let result = resolve_imports(
            &mut program,
            &tmp,
            &[],
            &mut visited,
            &mut in_progress,
            &mut module_exports,
        );
        assert!(result.is_ok());

        // Check that private_fn is NOT in the imported items
        let has_private = program.items.iter().any(|item| {
            if let ast::TopLevel::FnDef(fd) = item {
                fd.name == "private_fn"
            } else {
                false
            }
        });
        assert!(!has_private, "Private function should not be imported");

        // Check that public_fn IS in the imported items
        let has_public = program.items.iter().any(|item| {
            if let ast::TopLevel::FnDef(fd) = item {
                fd.name == "public_fn"
            } else {
                false
            }
        });
        assert!(has_public, "Public function should be imported");

        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_stdlib_import_resolution() {
        // Test that import String resolves from a non-stdlib directory
        let tmp = std::env::temp_dir().join("japl_test_stdlib_resolve");
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("test.japl"), "import String\nfn main() { println(concat(\"a\", \"b\")) }").unwrap();

        let stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../stdlib");
        if stdlib.exists() {
            let mut program = parse_file(&tmp.join("test.japl")).unwrap();
            let mut visited = HashSet::new();
            let mut in_progress = Vec::new();
            let mut module_exports = HashMap::new();
            let result = resolve_imports(
                &mut program,
                &tmp,
                &[stdlib],
                &mut visited,
                &mut in_progress,
                &mut module_exports,
            );
            assert!(result.is_ok(), "Should resolve stdlib import: {:?}", result);

            // Check that concat (a pub fn in String.japl) was imported
            let has_concat = program.items.iter().any(|item| {
                if let ast::TopLevel::FnDef(fd) = item {
                    fd.name == "concat"
                } else {
                    false
                }
            });
            assert!(has_concat, "Should have imported concat from String.japl");
        }

        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }
}

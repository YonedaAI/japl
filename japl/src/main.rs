use clap::{Parser, Subcommand};
use std::process::Command;

mod compiler;
mod runtime;
mod serve;

#[derive(Parser)]
#[command(name = "japl", about = "JAPL -- A typed actor language for distributed systems")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .japl file to .wasm
    Build {
        file: String,
        #[arg(long, default_value = "build")]
        out: String,
        /// Target: "local" (default) or "component" (Component Model canonical ABI)
        #[arg(long, default_value = "local")]
        target: String,
        /// Path to stdlib directory (default: auto-detected relative to binary or cwd)
        #[arg(long)]
        stdlib_path: Option<String>,
    },
    /// Compile and run a .japl file
    Run {
        file: String,
    },
    /// Compile and serve a .japl file over HTTP
    Serve {
        file: String,
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Type-check a .japl file
    Check {
        file: String,
    },
    /// Format a .japl file
    Fmt {
        file: String,
    },
    /// Deploy a JAPL HTTP app (compile, start NATS + wasmCloud, serve)
    Deploy {
        file: String,
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Target: "local" (default) or "component" (Component Model canonical ABI)
        #[arg(long, default_value = "local")]
        target: String,
    },
    /// Print version
    Version,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { file, out, target, stdlib_path } => {
            match compiler::compile_full(&file, &out, &target, stdlib_path.as_deref()) {
                Ok(wasm_path) => {
                    println!("{}", wasm_path);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Run { file } => {
            // Compile to temp directory, then run
            let tmp_dir = std::env::temp_dir().join("japl_build");
            let tmp_str = tmp_dir.display().to_string();
            match compiler::compile(&file, &tmp_str) {
                Ok(wasm_path) => {
                    if let Err(e) = runtime::run(&wasm_path) {
                        eprintln!("Runtime error: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Serve { file, port } => {
            let tmp_dir = std::env::temp_dir().join("japl_build");
            let tmp_str = tmp_dir.display().to_string();
            match compiler::compile(&file, &tmp_str) {
                Ok(wasm_path) => {
                    if let Err(e) = serve::serve(&wasm_path, port) {
                        eprintln!("Serve error: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Check { file } => {
            match compiler::check(&file) {
                Ok(()) => {}
                Err(_) => {
                    // Errors already printed by check()
                    std::process::exit(1);
                }
            }
        }
        Commands::Fmt { file } => {
            match compiler::format(&file) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Deploy { file, port, target } => {
            deploy(&file, port, &target);
        }
        Commands::Version => {
            println!("japl 1.0.0");
        }
    }
}

fn is_process_running(name: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ensure_nats() -> Result<(), String> {
    if is_process_running("nats-server") {
        eprintln!("[deploy] nats-server already running");
        return Ok(());
    }
    eprintln!("[deploy] Starting nats-server...");
    let child = Command::new("nats-server")
        .arg("-js")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    match child {
        Ok(_) => {
            // Give NATS a moment to bind
            std::thread::sleep(std::time::Duration::from_millis(500));
            eprintln!("[deploy] nats-server started (with JetStream)");
            Ok(())
        }
        Err(e) => Err(format!(
            "Failed to start nats-server: {}. Install with: brew install nats-server",
            e
        )),
    }
}

fn ensure_wasmcloud() -> Result<(), String> {
    // Check if wasmCloud host is running (wash-related processes)
    let wash_running = Command::new("wash")
        .args(["get", "hosts"])
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).contains("No hosts"))
        .unwrap_or(false);

    if wash_running {
        eprintln!("[deploy] wasmCloud host already running");
        return Ok(());
    }

    eprintln!("[deploy] Starting wasmCloud host...");
    // Try `wash up --detached` first (wash <= 0.28), then `wash start` (newer versions)
    let child = Command::new("wash")
        .args(["up", "--detached"])
        .output()
        .or_else(|_| Command::new("wash").args(["start"]).output());

    match child {
        Ok(output) => {
            if output.status.success() {
                // Give host time to initialize
                std::thread::sleep(std::time::Duration::from_secs(2));
                eprintln!("[deploy] wasmCloud host started");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // wash up may fail if already running, that's fine
                if stderr.contains("already") || stderr.contains("running") {
                    eprintln!("[deploy] wasmCloud host already running");
                    Ok(())
                } else {
                    Err(format!("wash up failed: {}", stderr))
                }
            }
        }
        Err(e) => {
            eprintln!("[deploy] wash not found ({}), skipping wasmCloud host", e);
            eprintln!("[deploy] Install with: curl -s https://raw.githubusercontent.com/wasmCloud/wasmCloud/main/crates/wash-cli/install.sh | bash");
            Ok(()) // Non-fatal: we can still serve directly
        }
    }
}

fn deploy(file: &str, port: u16, target: &str) {
    eprintln!("[deploy] Compiling {}...", file);

    // Step 1: Compile JAPL to core WASM
    let tmp_dir = std::env::temp_dir().join("japl_deploy");
    let tmp_str = tmp_dir.display().to_string();
    let wasm_path = match compiler::compile_with_target(file, &tmp_str, target) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
            std::process::exit(1);
        }
    };
    eprintln!("[deploy] Compiled to {}", wasm_path);

    // Step 2: Start NATS if not running
    if let Err(e) = ensure_nats() {
        eprintln!("[deploy] Warning: {}", e);
    }

    // Step 3: Start wasmCloud host if not running
    if let Err(e) = ensure_wasmcloud() {
        eprintln!("[deploy] Warning: {}", e);
    }

    // Step 4: Serve the compiled WASM via japl serve (provides host functions + HTTP)
    eprintln!("[deploy] Starting HTTP server on port {}...", port);
    eprintln!("[deploy] URL: http://localhost:{}", port);
    eprintln!("[deploy] NATS: nats://localhost:4222");
    eprintln!("[deploy] Press Ctrl+C to stop");
    eprintln!();

    if let Err(e) = serve::serve(&wasm_path, port) {
        eprintln!("Serve error: {}", e);
        std::process::exit(1);
    }
}

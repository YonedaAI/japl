use clap::{Parser, Subcommand};
use std::process::Command;

mod compiler;
mod package;
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
    /// Compile and run a .japl file locally (dev mode)
    Run {
        file: String,
        /// Distribution node name (enables clustering)
        #[arg(long)]
        node_name: Option<String>,
        /// Port to listen for peer connections
        #[arg(long, default_value = "9000")]
        listen_port: u16,
        /// Connect to a peer at host:port
        #[arg(long)]
        peer: Option<String>,
        /// Cluster cookie for authentication
        #[arg(long)]
        cookie: Option<String>,
    },
    /// Compile and serve a .japl file over HTTP locally (dev mode)
    Serve {
        file: String,
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Type-check a .japl file
    Check {
        file: String,
        /// Enable strict mode: warns on Pid/Int implicit conversions
        #[arg(long)]
        strict: bool,
    },
    /// Format a .japl file
    Fmt {
        file: String,
    },
    /// Deploy a JAPL app to wasmCloud (requires NATS + wasmCloud host)
    Deploy {
        file: String,
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Target: "local" (default) or "component" (Component Model canonical ABI)
        #[arg(long, default_value = "local")]
        target: String,
        /// Skip wasmCloud and serve locally instead (no infrastructure needed)
        #[arg(long)]
        local: bool,
        /// Print the generated WADM manifest without deploying
        #[arg(long)]
        dry_run: bool,
    },
    /// Show cluster node status
    Cluster {
        /// Node name
        #[arg(long)]
        node_name: Option<String>,
        /// Port for the distribution node
        #[arg(long, default_value = "9000")]
        port: u16,
    },
    /// Initialize a new JAPL project (creates japl.toml)
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Show project dependencies from japl.toml
    Deps,
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
        Commands::Run { file, node_name, listen_port, peer, cookie } => {
            // Compile to temp directory, then run
            let tmp_dir = std::env::temp_dir().join("japl_build");
            let tmp_str = tmp_dir.display().to_string();
            match compiler::compile(&file, &tmp_str) {
                Ok(wasm_path) => {
                    // If any distribution flag is set, create and start DistributionNode
                    let _dist_node = if node_name.is_some() || peer.is_some() {
                        let host = "127.0.0.1";
                        let node = runtime::distribution::DistributionNode::new(
                            host, listen_port, node_name.as_deref(), cookie.as_deref(),
                        );
                        if let Err(e) = node.listen() {
                            eprintln!("dist listen error: {}", e);
                        }
                        if let Some(peer_addr) = &peer {
                            if let Err(e) = node.connect_to(peer_addr) {
                                eprintln!("dist connect error: {}", e);
                            }
                        }
                        Some(node)
                    } else {
                        None
                    };
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
        Commands::Check { file, strict } => {
            match compiler::check(&file, strict) {
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
        Commands::Cluster { node_name, port } => {
            let host = "127.0.0.1";
            let node = runtime::distribution::DistributionNode::new(
                host, port, node_name.as_deref(), None,
            );
            println!("Node ID: {}", node.node_id());
            println!("Peers: {:?}", node.connected_peers());
        }
        Commands::Deploy { file, port, target, local, dry_run } => {
            deploy(&file, port, &target, local, dry_run);
        }
        Commands::Init { path } => {
            let dir = std::path::Path::new(&path).canonicalize().unwrap_or_else(|_| {
                std::path::PathBuf::from(&path)
            });
            match package::init_manifest(&dir) {
                Ok(manifest_path) => {
                    println!("Created {}", manifest_path);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Deps => {
            match package::read_manifest("japl.toml") {
                Ok(manifest) => {
                    println!("Package: {} v{}", manifest.name, manifest.version);
                    for (name, version) in &manifest.dependencies {
                        println!("  {} = {}", name, version);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
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
            Err(format!(
                "wash not found ({}). Install with: curl -s https://raw.githubusercontent.com/wasmCloud/wasmCloud/main/crates/wash-cli/install.sh | bash",
                e
            ))
        }
    }
}

fn extract_app_name(file: &str) -> String {
    std::path::Path::new(file)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn generate_wadm_manifest(app_name: &str, wasm_path: &str) -> String {
    let abs_wasm = std::path::Path::new(wasm_path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(wasm_path));
    // NOTE: The japl-provider is a standalone NATS sidecar, not a native
    // wasmCloud capability provider. The manifest below declares the component
    // only. The provider must be started separately:
    //   cd japl-provider && cargo run
    // Future work: convert japl-provider to use wasmcloud-provider-sdk so it
    // can be deployed as a real wasmCloud capability.
    format!(
        r#"apiVersion: core.oam.dev/v1beta1
kind: Application
metadata:
  name: {app_name}
  annotations:
    version: v0.1.0
    description: "JAPL application deployed via japl deploy"
    japl.provider.required: "true"
    japl.provider.type: "nats-sidecar"
    japl.provider.startup: "cd japl-provider && cargo run"
spec:
  components:
    - name: {app_name}
      type: component
      properties:
        image: file://{wasm_path}
      traits:
        - type: spreadscaler
          properties:
            instances: 1
"#,
        app_name = app_name,
        wasm_path = abs_wasm.display(),
    )
}

fn deploy(file: &str, port: u16, _target: &str, local: bool, dry_run: bool) {
    eprintln!("[deploy] Compiling {}...", file);

    // Step 1: Compile JAPL to WASM component (always use component target for deploy)
    let tmp_dir = std::env::temp_dir().join("japl_deploy");
    let tmp_str = tmp_dir.display().to_string();
    let wasm_path = match compiler::compile_with_target(file, &tmp_str, "component") {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
            std::process::exit(1);
        }
    };
    eprintln!("[deploy] Compiled to {}", wasm_path);

    // If --dry-run flag is set, print the manifest and exit
    if dry_run {
        let app_name = extract_app_name(file);
        let manifest = generate_wadm_manifest(&app_name, &wasm_path);
        println!("{}", manifest);
        return;
    }

    // If --local flag is set, skip wasmCloud and serve directly
    if local {
        eprintln!("[deploy] --local mode: serving directly on port {}", port);
        eprintln!("[deploy] URL: http://localhost:{}", port);
        eprintln!("[deploy] Press Ctrl+C to stop");
        eprintln!();
        if let Err(e) = serve::serve(&wasm_path, port) {
            eprintln!("Serve error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Step 2: Start NATS if not running
    if let Err(e) = ensure_nats() {
        eprintln!("[deploy] ERROR: {}", e);
        eprintln!("[deploy] NATS is required for wasmCloud deployment.");
        eprintln!("[deploy] Install: brew install nats-server && nats-server -js");
        eprintln!("[deploy] Or use --local: japl deploy --local {}", file);
        std::process::exit(1);
    }

    // Step 3: Start wasmCloud host if not running
    if let Err(e) = ensure_wasmcloud() {
        eprintln!("[deploy] ERROR: {}", e);
        eprintln!("[deploy] wasmCloud is required for deployment.");
        eprintln!("[deploy] Install wash: curl -s https://raw.githubusercontent.com/wasmCloud/wasmCloud/main/crates/wash-cli/install.sh | bash");
        eprintln!("[deploy] Start wasmCloud: wash up --detached");
        eprintln!("[deploy] Or use --local: japl deploy --local {}", file);
        std::process::exit(1);
    }

    // Step 4: Generate and write WADM manifest
    let app_name = extract_app_name(file);
    let manifest = generate_wadm_manifest(&app_name, &wasm_path);
    let manifest_path = format!("{}/{}.wadm.yaml", tmp_str, app_name);
    std::fs::create_dir_all(&tmp_str).ok();
    if let Err(e) = std::fs::write(&manifest_path, &manifest) {
        eprintln!("[deploy] Failed to write manifest: {}", e);
        std::process::exit(1);
    }
    eprintln!("[deploy] Generated WADM manifest: {}", manifest_path);

    // Step 5: Deploy via wash app deploy
    let deploy_result = Command::new("wash")
        .args(["app", "deploy", &manifest_path])
        .output();

    match deploy_result {
        Ok(output) if output.status.success() => {
            eprintln!("[deploy] Application '{}' deployed to wasmCloud", app_name);
            eprintln!("[deploy] Manifest: {}", manifest_path);
            eprintln!("[deploy] Check status: wash app list");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("[deploy] ERROR: wasmCloud deployment failed: {}", stderr);
            eprintln!("[deploy] ");
            eprintln!("[deploy] To fix:");
            eprintln!("[deploy]   1. Install wash: curl -s https://raw.githubusercontent.com/wasmCloud/wasmCloud/main/crates/wash-cli/install.sh | bash");
            eprintln!("[deploy]   2. Start wasmCloud: wash up --detached");
            eprintln!("[deploy]   3. Start NATS: nats-server -js");
            eprintln!("[deploy] ");
            eprintln!("[deploy] Or use --local for local-only serving: japl deploy --local {}", file);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("[deploy] ERROR: wasmCloud deployment failed: {}", e);
            eprintln!("[deploy] ");
            eprintln!("[deploy] To fix:");
            eprintln!("[deploy]   1. Install wash: curl -s https://raw.githubusercontent.com/wasmCloud/wasmCloud/main/crates/wash-cli/install.sh | bash");
            eprintln!("[deploy]   2. Start wasmCloud: wash up --detached");
            eprintln!("[deploy]   3. Start NATS: nats-server -js");
            eprintln!("[deploy] ");
            eprintln!("[deploy] Or use --local for local-only serving: japl deploy --local {}", file);
            std::process::exit(1);
        }
    }
}

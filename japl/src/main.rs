use clap::{Parser, Subcommand};

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
    /// Print version
    Version,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { file, out } => {
            match compiler::compile(&file, &out) {
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
                Err(e) => {
                    eprintln!("{}", e);
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
        Commands::Version => {
            println!("japl 1.0.0");
        }
    }
}

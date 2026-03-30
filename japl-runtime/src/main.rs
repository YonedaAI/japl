mod engine;
mod host;
mod process;
mod scheduler;

use clap::Parser;
use scheduler::Scheduler;

#[derive(Parser)]
#[command(name = "japl-runtime", about = "JAPL process runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Run a WASM module
    Run {
        /// Path to .wasm file
        file: String,

        /// Node name (for distributed mode)
        #[arg(long)]
        node: Option<String>,

        /// Listen address
        #[arg(long)]
        listen: Option<String>,

        /// Connect to peer
        #[arg(long)]
        connect: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            node: _,
            listen: _,
            connect: _,
        } => {
            let mut scheduler = Scheduler::new();
            scheduler.load_module(&file)?;
            scheduler.run()?;
        }
    }

    Ok(())
}

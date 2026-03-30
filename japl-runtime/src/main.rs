mod distribution;
mod engine;
mod host;
mod node;
mod process;
mod scheduler;
mod wire;

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

        /// Listen address (e.g. ":9000")
        #[arg(long)]
        listen: Option<String>,

        /// Connect to peer (e.g. "localhost:9000")
        #[arg(long)]
        connect: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            node,
            listen,
            connect,
        } => {
            let mut scheduler = Scheduler::new();

            if let Some(node_name) = node {
                let dist = distribution::DistributionLayer::new(
                    node_name.clone(),
                    "japl-default-cookie".to_string(),
                    scheduler.command_sender(),
                );

                if let Some(addr) = listen {
                    dist.listen(&addr)?;
                }

                if let Some(peer) = connect {
                    dist.connect(&peer)?;
                }

                scheduler.set_distribution(dist);
                println!("[{}] Distribution layer active", node_name);
            }

            scheduler.load_module(&file)?;
            scheduler.run()?;
        }
    }

    Ok(())
}

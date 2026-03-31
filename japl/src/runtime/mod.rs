pub mod engine;
pub mod host;
pub mod process;
pub mod scheduler;

use scheduler::Scheduler;

/// Run a WASM module with the JAPL runtime.
pub fn run(wasm_path: &str) -> Result<(), anyhow::Error> {
    let mut scheduler = Scheduler::new();
    scheduler.load_module(wasm_path)?;
    scheduler.run()?;
    Ok(())
}

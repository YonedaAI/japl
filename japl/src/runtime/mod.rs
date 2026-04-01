pub mod distribution;
pub mod distributed_host;
pub mod engine;
pub mod host;
pub mod process;
pub mod scheduler;
pub mod wire;

use scheduler::Scheduler;

/// Run a WASM module with the JAPL runtime (local mode).
pub fn run(wasm_path: &str) -> Result<(), anyhow::Error> {
    let mut scheduler = Scheduler::new();
    scheduler.load_module(wasm_path)?;
    scheduler.run()?;
    Ok(())
}

/// Run a WASM module with distributed host functions (NATS mode).
/// Process operations (spawn/send/receive) are routed to the JAPL provider
/// over NATS instead of the local scheduler.
/// If http_port is Some, an HTTP gateway is started on that port.
pub fn run_distributed(wasm_path: &str, nats_url: &str, http_port: Option<u16>) -> Result<(), anyhow::Error> {
    distributed_host::run_distributed(wasm_path, nats_url, http_port)
}

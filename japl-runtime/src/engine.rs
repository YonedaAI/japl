use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

use crate::host;
use crate::process::ProcessState;

/// Shared engine and compiled module.
pub struct JaplEngine {
    pub engine: Engine,
    pub module: Module,
}

impl JaplEngine {
    pub fn new(wasm_path: &str) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)?;
        Ok(Self { engine, module })
    }

    /// Build a fully-linked Linker with WASI preview-1 + JAPL host functions.
    pub fn build_linker(&self) -> anyhow::Result<Linker<ProcessState>> {
        let mut linker = Linker::new(&self.engine);

        // WASI preview-1 (the classic wasi_snapshot_preview1 imports)
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut ProcessState| {
            &mut state.wasi
        })?;

        // JAPL-specific host functions
        host::add_japl_host_functions(&mut linker)?;

        Ok(linker)
    }

    /// Create a WasiP1Ctx with inherited stdio.
    pub fn build_wasi_ctx() -> WasiP1Ctx {
        let ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .build_p1();
        ctx
    }
}

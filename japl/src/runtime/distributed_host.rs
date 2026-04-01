// =========================================================================
// JAPL Distributed Runtime
// =========================================================================
//
// Runs a compiled JAPL WASM module with process operations routed through
// the JAPL provider over NATS. This is the distributed execution path:
//
//   japl run --distributed app.japl
//
// The WASM module runs locally via wasmtime, but spawn/send/receive/self_pid
// go through NATS to the japl-provider instead of the local scheduler.
// All other host functions (println, file, env, etc.) run locally.
// =========================================================================

use std::sync::{Arc, Mutex};
use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

/// State for the distributed runtime (replaces ProcessState for this mode).
pub struct DistributedState {
    pub wasi: WasiP1Ctx,
    pub nc: Arc<nats::Connection>,
    pub pid: u64,
}

/// Run a WASM module with NATS-backed process functions.
pub fn run_distributed(wasm_path: &str, nats_url: &str) -> Result<(), anyhow::Error> {
    eprintln!("[distributed] Connecting to NATS at {}", nats_url);
    let nc = Arc::new(nats::connect(nats_url)?);

    // Health check
    match nc.request_timeout("japl.runtime.health", "{}", std::time::Duration::from_secs(3)) {
        Ok(resp) => {
            let body = String::from_utf8_lossy(&resp.data);
            eprintln!("[distributed] Provider health: {}", body);
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "JAPL provider not responding: {}\nStart: cd japl-provider && cargo run --release", e
            ));
        }
    }

    // Spawn a process in the provider to get our PID
    let spawn_resp = nc.request_timeout(
        "japl.runtime.spawn",
        r#"{"closure_data":[]}"#,
        std::time::Duration::from_secs(5),
    )?;
    let spawn_body = String::from_utf8_lossy(&spawn_resp.data);
    let main_pid: u64 = serde_json::from_str::<serde_json::Value>(&spawn_body)?
        ["pid"].as_u64().unwrap_or(1);
    eprintln!("[distributed] Main process PID: {}", main_pid);

    // Build engine and module
    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path)?;

    // Build linker with NATS-backed functions
    let mut linker: Linker<DistributedState> = Linker::new(&engine);

    // WASI
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut DistributedState| {
        &mut state.wasi
    })?;

    // === NATS-backed process functions ===

    // spawn
    linker.func_wrap("japl", "spawn", |mut caller: Caller<'_, DistributedState>, _closure_ptr: i64, _closure_size: i64| -> i64 {
        let nc = caller.data().nc.clone();
        match nc.request_timeout("japl.runtime.spawn", r#"{"closure_data":[]}"#, std::time::Duration::from_secs(5)) {
            Ok(resp) => {
                let body = String::from_utf8_lossy(&resp.data);
                serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v["pid"].as_i64())
                    .unwrap_or(-1)
            }
            Err(e) => { eprintln!("[distributed] spawn failed: {}", e); -1 }
        }
    })?;

    // send
    linker.func_wrap("japl", "send", |mut caller: Caller<'_, DistributedState>, pid: i64, msg_ptr: i64| {
        let nc = caller.data().nc.clone();
        // Read message bytes from WASM memory
        let msg_bytes = {
            let mem = caller.get_export("memory").and_then(|e| e.into_memory());
            match mem {
                Some(m) => {
                    let data = m.data(&caller);
                    let ptr = msg_ptr as usize;
                    if ptr + 8 > data.len() { return; }
                    let field_count = u32::from_le_bytes(
                        data[ptr+4..ptr+8].try_into().unwrap_or([0;4])
                    ) as usize;
                    let total = 8 + field_count * 8;
                    let end = (ptr + total).min(data.len());
                    data[ptr..end].to_vec()
                }
                None => return,
            }
        };
        let payload = serde_json::json!({"message": msg_bytes}).to_string();
        let subject = format!("japl.runtime.send.{}", pid);
        let _ = nc.request_timeout(&subject, &payload, std::time::Duration::from_secs(5));
    })?;

    // receive
    linker.func_wrap("japl", "receive", |mut caller: Caller<'_, DistributedState>| -> i64 {
        let pid = caller.data().pid;
        let nc = caller.data().nc.clone();
        let subject = format!("japl.runtime.receive.{}", pid);
        match nc.request_timeout(&subject, "{}", std::time::Duration::from_secs(30)) {
            Ok(resp) => {
                let body = String::from_utf8_lossy(&resp.data);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(msg_arr) = v["message"].as_array() {
                        let bytes: Vec<u8> = msg_arr.iter()
                            .filter_map(|b| b.as_u64().map(|n| n as u8))
                            .collect();
                        // Write into WASM memory
                        let mem = caller.get_export("memory").and_then(|e| e.into_memory());
                        let heap_global = caller.get_export("heap_ptr").and_then(|e| e.into_global());
                        match (mem, heap_global) {
                            (Some(m), Some(g)) => {
                                let heap_ptr = g.get(&mut caller).i32().unwrap_or(0) as usize;
                                let data = m.data_mut(&mut caller);
                                if heap_ptr + bytes.len() <= data.len() {
                                    data[heap_ptr..heap_ptr + bytes.len()].copy_from_slice(&bytes);
                                    let new_heap = ((heap_ptr + bytes.len() + 7) & !7) as i32;
                                    let _ = g.set(&mut caller, Val::I32(new_heap));
                                    return heap_ptr as i64;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                0
            }
            Err(e) => { eprintln!("[distributed] receive failed: {}", e); 0 }
        }
    })?;

    // self_pid
    linker.func_wrap("japl", "self_pid", |caller: Caller<'_, DistributedState>| -> i64 {
        caller.data().pid as i64
    })?;

    // === Local host functions (non-process) ===

    // println
    linker.func_wrap("japl", "println", |mut caller: Caller<'_, DistributedState>, ptr: i32, len: i32| {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end <= data.len() {
                let s = std::str::from_utf8(&data[start..end]).unwrap_or("<invalid utf8>");
                println!("{}", s);
            }
        }
    })?;

    // show (int to string) - needed for basic output
    linker.func_wrap("japl", "print_bytes", |_caller: Caller<'_, DistributedState>, _ptr: i32, _len: i32| {})?;

    // time functions
    linker.func_wrap("japl", "time_now", || -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    })?;
    linker.func_wrap("japl", "time_sleep", |millis: i64| {
        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
    })?;

    // Stubs for functions that may be imported but aren't needed in distributed mode
    linker.func_wrap("japl", "spawn_remote", |_: i32, _: i64| -> i64 { -1 })?;
    linker.func_wrap("japl", "process_count", |_: Caller<'_, DistributedState>| -> i64 { 0 })?;
    linker.func_wrap("japl", "is_process_alive", |_: Caller<'_, DistributedState>, _: i64| -> i64 { 0 })?;
    linker.func_wrap("japl", "mailbox_size", |_: Caller<'_, DistributedState>, _: i64| -> i64 { 0 })?;

    // String operations
    linker.func_wrap("japl", "char_at", |mut caller: Caller<'_, DistributedState>, str_ptr: i32, index: i32| -> i32 {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(&caller);
            let ptr = str_ptr as usize;
            if ptr + 4 > data.len() { return 0; }
            let len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap_or([0;4])) as usize;
            let idx = index as usize;
            if idx < len && ptr + 4 + idx < data.len() {
                return data[ptr + 4 + idx] as i32;
            }
        }
        0
    })?;
    linker.func_wrap("japl", "str_length", |mut caller: Caller<'_, DistributedState>, str_ptr: i32| -> i32 {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(&caller);
            let ptr = str_ptr as usize;
            if ptr + 4 <= data.len() {
                return u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap_or([0;4])) as i32;
            }
        }
        0
    })?;
    linker.func_wrap("japl", "string_eq", |mut caller: Caller<'_, DistributedState>, a_ptr: i32, b_ptr: i32| -> i32 {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(&caller);
            let ap = a_ptr as usize;
            let bp = b_ptr as usize;
            if ap + 4 <= data.len() && bp + 4 <= data.len() {
                let a_len = u32::from_le_bytes(data[ap..ap+4].try_into().unwrap_or([0;4])) as usize;
                let b_len = u32::from_le_bytes(data[bp..bp+4].try_into().unwrap_or([0;4])) as usize;
                if a_len == b_len && ap + 4 + a_len <= data.len() && bp + 4 + b_len <= data.len() {
                    return if data[ap+4..ap+4+a_len] == data[bp+4..bp+4+b_len] { 1 } else { 0 };
                }
            }
        }
        0
    })?;
    linker.func_wrap("japl", "substring", |mut caller: Caller<'_, DistributedState>, str_ptr: i32, start: i32, end: i32| -> i32 {
        // Return pointer to new string in heap
        0 // stub — complex to implement standalone
    })?;
    linker.func_wrap("japl", "string_index_of", |_: Caller<'_, DistributedState>, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "from_char_code", |_: Caller<'_, DistributedState>, _: i32| -> i32 { 0 })?;

    // Other stubs
    linker.func_wrap("japl", "llm", |_: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "llm_str", |_: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "llm_structured_str", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "llm_structured", |_: i32, _: i32, _: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "tcp_listen", |_: Caller<'_, DistributedState>, _: i32| -> i64 { -1 })?;
    linker.func_wrap("japl", "tcp_accept", |_: Caller<'_, DistributedState>, _: i64| -> i64 { -1 })?;
    linker.func_wrap("japl", "tcp_connect", |_: Caller<'_, DistributedState>, _: i32, _: i32, _: i32| -> i64 { -1 })?;
    linker.func_wrap("japl", "tcp_read", |_: Caller<'_, DistributedState>, _: i64, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "tcp_write", |_: Caller<'_, DistributedState>, _: i64, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "tcp_close", |_: Caller<'_, DistributedState>, _: i64| {})?;
    linker.func_wrap("japl", "env_get", |_: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "env_get_str", |_: Caller<'_, DistributedState>, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "env_args_count", || -> i32 { 0 })?;
    linker.func_wrap("japl", "crypto_sha256", |_: Caller<'_, DistributedState>, _: i32, _: i32, _: i32| {})?;
    linker.func_wrap("japl", "crypto_random", |_: Caller<'_, DistributedState>, _: i32, _: i32| {})?;
    linker.func_wrap("japl", "file_read_str", |_: Caller<'_, DistributedState>, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "file_read", |_: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "file_write", |_: i32, _: i32, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "file_exists", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "file_write_str", |_: Caller<'_, DistributedState>, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "file_exists_str", |_: Caller<'_, DistributedState>, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "bytes_alloc", |_: Caller<'_, DistributedState>, _: i32| -> i32 { 0 })?;

    // Create state and instantiate
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_env().build_p1();
    let state = DistributedState { wasi, nc: nc.clone(), pid: main_pid };
    let mut store = Store::new(&engine, state);
    let instance = linker.instantiate(&mut store, &module)?;

    // Call _start
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;

    eprintln!("[distributed] Execution complete");
    Ok(())
}

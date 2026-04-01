// =========================================================================
// JAPL Distributed Runtime
// =========================================================================
//
// Runs compiled JAPL WASM apps with process operations through NATS.
// WASM execution is local (wasmtime), process table is shared via NATS
// through the japl-provider. This mirrors Erlang's model: code runs
// locally on each node, messaging is distributed.
//
//   japl run --distributed app.japl
//
// Each spawned process gets:
//   1. A PID allocated by the provider (via NATS)
//   2. A local OS thread running a new WASM instance
//   3. Host functions routing spawn/send/receive through NATS
//
// =========================================================================

use std::sync::Arc;
use wasmtime::*;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

/// Shared state across all process instances.
struct SharedState {
    engine: Engine,
    module: Module,
    nc: nats::Connection,
}

/// Per-process state for the WASM Store.
pub struct DistributedState {
    pub wasi: WasiP1Ctx,
    pub nc: Arc<nats::Connection>,
    pub pid: u64,
    pub shared: Arc<SharedState>,
}

/// Build a linker with NATS-backed process functions and local non-process functions.
fn build_linker(engine: &Engine, shared: &Arc<SharedState>) -> Result<Linker<DistributedState>, anyhow::Error> {
    let mut linker: Linker<DistributedState> = Linker::new(engine);

    // WASI
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut DistributedState| {
        &mut state.wasi
    })?;

    // =========================================================
    // NATS-backed process functions
    // =========================================================

    // spawn: allocate PID via provider, then run closure in a new local thread
    linker.func_wrap("japl", "spawn", |mut caller: Caller<'_, DistributedState>, closure_ptr: i64, closure_size: i64| -> i64 {
        let nc = caller.data().nc.clone();
        let shared = caller.data().shared.clone();

        // 1. Read closure bytes from WASM memory
        let closure_bytes = {
            let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return -1,
            };
            let data = mem.data(&caller);
            let ptr = closure_ptr as usize;
            let size = if closure_size > 0 && (closure_size as usize) < 4096 {
                closure_size as usize
            } else {
                // Default: read header to determine size
                if ptr + 8 > data.len() { return -1; }
                let _table_idx = u64::from_le_bytes(data[ptr..ptr+8].try_into().unwrap_or([0;8]));
                256 // conservative closure size
            };
            let end = (ptr + size).min(data.len());
            data[ptr..end].to_vec()
        };

        // 2. Allocate PID from provider
        let pid: i64 = match nc.request_timeout(
            "japl.runtime.spawn",
            &serde_json::json!({"closure_data": closure_bytes}).to_string(),
            std::time::Duration::from_secs(5),
        ) {
            Ok(resp) => {
                let body = String::from_utf8_lossy(&resp.data);
                serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v["pid"].as_i64())
                    .unwrap_or(-1)
            }
            Err(e) => { eprintln!("[distributed] spawn: provider error: {}", e); return -1; }
        };
        if pid < 0 { return -1; }

        // 3. Spawn a local thread to run the closure in a new WASM instance
        let closure_data = closure_bytes.clone();
        let child_pid = pid as u64;

        std::thread::Builder::new()
            .name(format!("japl-dist-pid-{}", child_pid))
            .spawn(move || {
                if let Err(e) = run_process(shared, child_pid, closure_data) {
                    eprintln!("[distributed:pid-{}] process error: {}", child_pid, e);
                }
            })
            .expect("failed to spawn distributed process thread");

        pid
    })?;

    // send: route to provider via NATS
    linker.func_wrap("japl", "send", |mut caller: Caller<'_, DistributedState>, pid: i64, msg_ptr: i64| {
        let nc = caller.data().nc.clone();
        let msg_bytes = read_msg_bytes(&mut caller, msg_ptr);
        let payload = serde_json::json!({"message": msg_bytes}).to_string();
        let subject = format!("japl.runtime.send.{}", pid);
        let _ = nc.request_timeout(&subject, &payload, std::time::Duration::from_secs(5));
    })?;

    // receive: block on provider mailbox via NATS
    linker.func_wrap("japl", "receive", |mut caller: Caller<'_, DistributedState>| -> i64 {
        let pid = caller.data().pid;
        let nc = caller.data().nc.clone();
        let subject = format!("japl.runtime.receive.{}", pid);
        eprintln!("[distributed:pid-{}] receive: waiting on {}", pid, subject);
        match nc.request_timeout(&subject, "{}", std::time::Duration::from_secs(30)) {
            Ok(resp) => {
                let body = String::from_utf8_lossy(&resp.data);
                eprintln!("[distributed:pid-{}] receive: got {} bytes: {}", pid, resp.data.len(), &body[..body.len().min(100)]);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(msg_arr) = v["message"].as_array() {
                        let bytes: Vec<u8> = msg_arr.iter()
                            .filter_map(|b| b.as_u64().map(|n| n as u8))
                            .collect();
                        eprintln!("[distributed:pid-{}] receive: decoded {} bytes", pid, bytes.len());
                        return write_bytes_to_heap(&mut caller, &bytes);
                    } else {
                        eprintln!("[distributed:pid-{}] receive: no 'message' array in response", pid);
                    }
                } else {
                    eprintln!("[distributed:pid-{}] receive: invalid JSON: {}", pid, body);
                }
                0
            }
            Err(e) => { eprintln!("[distributed:pid-{}] receive timeout: {}", pid, e); 0 }
        }
    })?;

    // self_pid
    linker.func_wrap("japl", "self_pid", |caller: Caller<'_, DistributedState>| -> i64 {
        caller.data().pid as i64
    })?;

    // =========================================================
    // Local non-process host functions
    // =========================================================

    // println
    linker.func_wrap("japl", "println", |mut caller: Caller<'_, DistributedState>, ptr: i32, len: i32| {
        if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(&caller);
            let s = ptr as usize;
            let e = s + len as usize;
            if e <= data.len() {
                let text = std::str::from_utf8(&data[s..e]).unwrap_or("<invalid>");
                println!("{}", text);
            }
        }
    })?;

    // time
    linker.func_wrap("japl", "time_now", || -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64
    })?;
    linker.func_wrap("japl", "time_sleep", |millis: i64| {
        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
    })?;

    // Stubs for non-critical host functions
    linker.func_wrap("japl", "spawn_remote", |_: i32, _: i64| -> i64 { -1 })?;
    linker.func_wrap("japl", "process_count", |_: Caller<'_, DistributedState>| -> i64 { 0 })?;
    linker.func_wrap("japl", "is_process_alive", |_: Caller<'_, DistributedState>, _: i64| -> i64 { 0 })?;
    linker.func_wrap("japl", "mailbox_size", |_: Caller<'_, DistributedState>, _: i64| -> i64 { 0 })?;
    linker.func_wrap("japl", "print_bytes", |_: Caller<'_, DistributedState>, _: i32, _: i32| {})?;
    linker.func_wrap("japl", "char_at", |mut caller: Caller<'_, DistributedState>, str_ptr: i32, index: i32| -> i32 {
        read_char_at(&mut caller, str_ptr, index)
    })?;
    linker.func_wrap("japl", "str_length", |mut caller: Caller<'_, DistributedState>, str_ptr: i32| -> i32 {
        read_str_length(&mut caller, str_ptr)
    })?;
    linker.func_wrap("japl", "string_eq", |mut caller: Caller<'_, DistributedState>, a: i32, b: i32| -> i32 {
        string_eq_impl(&mut caller, a, b)
    })?;
    linker.func_wrap("japl", "substring", |_: Caller<'_, DistributedState>, _: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "string_index_of", |_: Caller<'_, DistributedState>, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "from_char_code", |_: Caller<'_, DistributedState>, _: i32| -> i32 { 0 })?;
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

    Ok(linker)
}

/// Run a spawned process: load a fresh WASM instance and call __process_entry(closure_ptr).
fn run_process(shared: Arc<SharedState>, pid: u64, closure_data: Vec<u8>) -> Result<(), anyhow::Error> {
    let linker = build_linker(&shared.engine, &shared)?;
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_env().build_p1();
    let nc = Arc::new(shared.nc.clone());
    let state = DistributedState { wasi, nc, pid, shared: shared.clone() };
    let mut store = Store::new(&shared.engine, state);
    let instance = linker.instantiate(&mut store, &shared.module)?;

    // Write closure data into the new instance's memory
    let mem = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow::anyhow!("no memory export"))?;
    let heap_global = instance.get_global(&mut store, "heap_ptr")
        .ok_or_else(|| anyhow::anyhow!("no heap_ptr export"))?;
    let heap_ptr = heap_global.get(&mut store).i32().unwrap_or(0) as usize;

    let data = mem.data_mut(&mut store);
    if heap_ptr + closure_data.len() <= data.len() {
        data[heap_ptr..heap_ptr + closure_data.len()].copy_from_slice(&closure_data);
    }

    let closure_ptr = heap_ptr as i64;
    let new_heap = ((heap_ptr + closure_data.len() + 7) & !7) as i32;
    let _ = heap_global.set(&mut store, Val::I32(new_heap));

    // Call __process_entry if it exists, otherwise _start
    if let Ok(entry) = instance.get_typed_func::<i64, ()>(&mut store, "__process_entry") {
        entry.call(&mut store, closure_ptr)?;
    } else if let Ok(start) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
        start.call(&mut store, ())?;
    }

    Ok(())
}

/// Start an HTTP gateway that bridges REST requests to NATS-backed JAPL processes.
/// Routes: PUT /kv/{key}/{val}, GET /kv/{key}, DELETE /kv/{key}, GET /health
fn start_http_gateway(port: u16, nats_url: &str, service_pid: u64) -> Result<(), anyhow::Error> {
    let nc = nats::connect(nats_url)?;
    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("HTTP bind failed: {}", e))?;
    eprintln!("[http-gateway] Listening on http://localhost:{}", port);
    eprintln!("[http-gateway] Routes: PUT /kv/{{key}}/{{val}}, GET /kv/{{key}}, DELETE /kv/{{key}}, GET /health");

    // Allocate a gateway PID for receiving NATS replies
    let spawn_resp = nc.request_timeout("japl.runtime.spawn", r#"{"closure_data":[]}"#, std::time::Duration::from_secs(5))?;
    let gw_pid: u64 = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&spawn_resp.data))?
        ["pid"].as_u64().unwrap_or(0);
    eprintln!("[http-gateway] Gateway PID: {}", gw_pid);

    std::thread::Builder::new()
        .name("http-gateway".into())
        .spawn(move || {
            for request in server.incoming_requests() {
                let method = request.method().to_string();
                let url = request.url().to_string();
                let parts: Vec<&str> = url.trim_start_matches('/').split('/').collect();

                let response = match (method.as_str(), parts.as_slice()) {
                    ("GET", ["health"]) => {
                        match nc.request_timeout("japl.runtime.health", "{}", std::time::Duration::from_secs(3)) {
                            Ok(resp) => String::from_utf8_lossy(&resp.data).to_string(),
                            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                        }
                    }
                    ("PUT", ["kv", key_str, val_str]) => {
                        let key: i64 = key_str.parse().unwrap_or(0);
                        let val: i64 = val_str.parse().unwrap_or(0);
                        // Build CmdPut(key, val, gw_pid) — tag=0, 3 fields
                        let msg: Vec<u8> = [
                            0u32.to_le_bytes().to_vec(), 3u32.to_le_bytes().to_vec(),
                            key.to_le_bytes().to_vec(), val.to_le_bytes().to_vec(),
                            (gw_pid as i64).to_le_bytes().to_vec(),
                        ].concat();
                        let payload = serde_json::json!({"message": msg}).to_string();
                        let _ = nc.request_timeout(
                            &format!("japl.runtime.send.{}", service_pid),
                            &payload, std::time::Duration::from_secs(5)
                        );
                        // Wait for reply
                        match nc.request_timeout(
                            &format!("japl.runtime.receive.{}", gw_pid),
                            "{}", std::time::Duration::from_secs(5)
                        ) {
                            Ok(resp) => {
                                let body = String::from_utf8_lossy(&resp.data);
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                                    if let Some(arr) = v["message"].as_array() {
                                        let bytes: Vec<u8> = arr.iter()
                                            .filter_map(|b| b.as_u64().map(|n| n as u8))
                                            .collect();
                                        if bytes.len() >= 8 {
                                            let tag = u32::from_le_bytes(bytes[0..4].try_into().unwrap_or([0;4]));
                                            if tag == 0 { format!(r#"{{"status":"ok","key":{},"val":{}}}"#, key, val) }
                                            else { format!(r#"{{"status":"error","tag":{}}}"#, tag) }
                                        } else { r#"{"status":"error","reason":"short reply"}"#.to_string() }
                                    } else { r#"{"status":"error","reason":"no message"}"#.to_string() }
                                } else { r#"{"status":"error","reason":"invalid json"}"#.to_string() }
                            }
                            Err(_) => r#"{"status":"error","reason":"timeout"}"#.to_string(),
                        }
                    }
                    ("GET", ["kv", key_str]) => {
                        let key: i64 = key_str.parse().unwrap_or(0);
                        // Build CmdGet(key, gw_pid) — tag=1, 2 fields
                        let msg: Vec<u8> = [
                            1u32.to_le_bytes().to_vec(), 2u32.to_le_bytes().to_vec(),
                            key.to_le_bytes().to_vec(), (gw_pid as i64).to_le_bytes().to_vec(),
                        ].concat();
                        let payload = serde_json::json!({"message": msg}).to_string();
                        let _ = nc.request_timeout(
                            &format!("japl.runtime.send.{}", service_pid),
                            &payload, std::time::Duration::from_secs(5)
                        );
                        match nc.request_timeout(
                            &format!("japl.runtime.receive.{}", gw_pid),
                            "{}", std::time::Duration::from_secs(5)
                        ) {
                            Ok(resp) => {
                                let body = String::from_utf8_lossy(&resp.data);
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                                    if let Some(arr) = v["message"].as_array() {
                                        let bytes: Vec<u8> = arr.iter()
                                            .filter_map(|b| b.as_u64().map(|n| n as u8))
                                            .collect();
                                        if bytes.len() >= 8 {
                                            let tag = u32::from_le_bytes(bytes[0..4].try_into().unwrap_or([0;4]));
                                            match tag {
                                                1 => { // ReplyValue
                                                    let val = i64::from_le_bytes(bytes[8..16].try_into().unwrap_or([0;8]));
                                                    format!(r#"{{"key":{},"value":{}}}"#, key, val)
                                                }
                                                2 => format!(r#"{{"key":{},"error":"not_found"}}"#, key),
                                                _ => format!(r#"{{"error":"unknown_tag","tag":{}}}"#, tag),
                                            }
                                        } else { format!(r#"{{"error":"short_reply"}}"#) }
                                    } else { format!(r#"{{"error":"no_message"}}"#) }
                                } else { format!(r#"{{"error":"bad_json"}}"#) }
                            }
                            Err(_) => format!(r#"{{"key":{},"error":"timeout"}}"#, key),
                        }
                    }
                    ("DELETE", ["kv", key_str]) => {
                        let key: i64 = key_str.parse().unwrap_or(0);
                        // Build CmdDel(key, gw_pid) — tag=2, 2 fields
                        let msg: Vec<u8> = [
                            2u32.to_le_bytes().to_vec(), 2u32.to_le_bytes().to_vec(),
                            key.to_le_bytes().to_vec(), (gw_pid as i64).to_le_bytes().to_vec(),
                        ].concat();
                        let payload = serde_json::json!({"message": msg}).to_string();
                        let _ = nc.request_timeout(
                            &format!("japl.runtime.send.{}", service_pid),
                            &payload, std::time::Duration::from_secs(5)
                        );
                        match nc.request_timeout(
                            &format!("japl.runtime.receive.{}", gw_pid),
                            "{}", std::time::Duration::from_secs(5)
                        ) {
                            Ok(_) => format!(r#"{{"status":"ok","key":{},"deleted":true}}"#, key),
                            Err(_) => format!(r#"{{"key":{},"error":"timeout"}}"#, key),
                        }
                    }
                    _ => r#"{"error":"unknown route","routes":["GET /health","PUT /kv/{key}/{val}","GET /kv/{key}","DELETE /kv/{key}"]}"#.to_string(),
                };

                let resp = tiny_http::Response::from_string(&response)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                let _ = request.respond(resp);
            }
        })?;

    Ok(())
}

/// Run a WASM module with NATS-backed process functions.
pub fn run_distributed(wasm_path: &str, nats_url: &str, http_port: Option<u16>) -> Result<(), anyhow::Error> {
    eprintln!("[distributed] Connecting to NATS at {}", nats_url);
    let nc = nats::connect(nats_url)?;

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

    // Allocate main process PID
    let spawn_resp = nc.request_timeout("japl.runtime.spawn", r#"{"closure_data":[]}"#, std::time::Duration::from_secs(5))?;
    let main_pid: u64 = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&spawn_resp.data))?
        ["pid"].as_u64().unwrap_or(1);
    eprintln!("[distributed] Main process PID: {}", main_pid);

    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path)?;

    // Start HTTP gateway if requested (before JAPL app starts, so the gateway
    // knows the service coordinator PID — it's the main_pid)
    if let Some(port) = http_port {
        start_http_gateway(port, nats_url, main_pid)?;
    }

    let shared = Arc::new(SharedState { engine: engine.clone(), module, nc });
    let linker = build_linker(&engine, &shared)?;

    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_env().build_p1();
    let nc_arc = Arc::new(shared.nc.clone());
    let state = DistributedState { wasi, nc: nc_arc, pid: main_pid, shared: shared.clone() };
    let mut store = Store::new(&engine, state);
    let instance = linker.instantiate(&mut store, &shared.module)?;

    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;

    // If HTTP gateway is running, keep the process alive
    if http_port.is_some() {
        eprintln!("[distributed] Service running with HTTP gateway. Press Ctrl+C to stop.");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    // Wait for spawned threads to finish
    std::thread::sleep(std::time::Duration::from_millis(500));

    eprintln!("[distributed] Execution complete");
    Ok(())
}

// =========================================================
// Helper functions
// =========================================================

fn read_msg_bytes(caller: &mut Caller<'_, DistributedState>, msg_ptr: i64) -> Vec<u8> {
    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(m) => m,
        None => return vec![],
    };
    let data = mem.data(caller);
    let ptr = msg_ptr as usize;
    if ptr + 8 > data.len() { return vec![]; }
    let field_count = u32::from_le_bytes(data[ptr+4..ptr+8].try_into().unwrap_or([0;4])) as usize;
    let total = 8 + field_count * 8;
    let end = (ptr + total).min(data.len());
    data[ptr..end].to_vec()
}

fn write_bytes_to_heap(caller: &mut Caller<'_, DistributedState>, bytes: &[u8]) -> i64 {
    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(m) => m,
        None => return 0,
    };
    let heap_global = match caller.get_export("heap_ptr").and_then(|e| e.into_global()) {
        Some(g) => g,
        None => return 0,
    };
    let heap_ptr = heap_global.get(&mut *caller).i32().unwrap_or(0) as usize;
    let data = mem.data_mut(&mut *caller);
    if heap_ptr + bytes.len() > data.len() { return 0; }
    data[heap_ptr..heap_ptr + bytes.len()].copy_from_slice(bytes);
    let new_heap = ((heap_ptr + bytes.len() + 7) & !7) as i32;
    let _ = heap_global.set(&mut *caller, Val::I32(new_heap));
    heap_ptr as i64
}

fn read_char_at(caller: &mut Caller<'_, DistributedState>, str_ptr: i32, index: i32) -> i32 {
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = mem.data(caller);
        let ptr = str_ptr as usize;
        if ptr + 4 > data.len() { return 0; }
        let len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap_or([0;4])) as usize;
        let idx = index as usize;
        if idx < len && ptr + 4 + idx < data.len() { return data[ptr + 4 + idx] as i32; }
    }
    0
}

fn read_str_length(caller: &mut Caller<'_, DistributedState>, str_ptr: i32) -> i32 {
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = mem.data(caller);
        let ptr = str_ptr as usize;
        if ptr + 4 <= data.len() {
            return u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap_or([0;4])) as i32;
        }
    }
    0
}

fn string_eq_impl(caller: &mut Caller<'_, DistributedState>, a_ptr: i32, b_ptr: i32) -> i32 {
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let data = mem.data(caller);
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
}

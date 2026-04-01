use std::io::Write;
use wasmtime::*;

/// Get memory from a Caller, trying "memory" first then "cm32p2_memory" (canonical ABI name).
fn get_caller_memory<T>(caller: &mut Caller<'_, T>) -> Option<Memory> {
    caller.get_export("memory").and_then(|e| e.into_memory())
        .or_else(|| caller.get_export("cm32p2_memory").and_then(|e| e.into_memory()))
}

/// Get memory from an Instance, trying "memory" first then "cm32p2_memory".
fn get_instance_memory(store: &mut Store<()>, instance: &Instance) -> Option<Memory> {
    instance.get_memory(&mut *store, "memory")
        .or_else(|| instance.get_memory(&mut *store, "cm32p2_memory"))
}

/// Run the JAPL HTTP server for a compiled .wasm module.
pub fn serve(wasm_path: &str, port: u16) -> Result<(), anyhow::Error> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path)
        .map_err(|e| e.context(format!("Failed to load WASM module: {}", wasm_path)))?;

    let mut linker = Linker::<()>::new(&engine);

    // Register all host functions
    register_host_functions(&mut linker)?;

    // Create store and instantiate
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)
        .map_err(|e| e.context("Failed to instantiate WASM module"))?;

    // Call _start to initialize globals/data
    if let Some(start_fn) = instance.get_func(&mut store, "_start") {
        start_fn.call(&mut store, &[], &mut [])?;
    }

    // Verify __handle_http exists
    instance.get_typed_func::<(i32, i32, i32, i32, i32, i32), (i32, i32)>(&mut store, "__handle_http")
        .map_err(|e| e.context("WASM module does not export __handle_http — did you define fn handle_request(method, path, body)?"))?;

    println!("japl serve listening on http://0.0.0.0:{}", port);

    let server = tiny_http::Server::http(format!("0.0.0.0:{}", port))
        .map_err(|e| anyhow::anyhow!("Failed to bind HTTP server: {}", e))?;

    for mut request in server.incoming_requests() {
        let method = request.method().to_string();
        let path = request.url().to_string();

        // Read body
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap_or(0);

        match handle_wasm_request(&instance, &mut store, &method, &path, &body) {
            Ok(response_body) => {
                let response = tiny_http::Response::from_string(&response_body)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap(),
                    );
                let _ = request.respond(response);
            }
            Err(e) => {
                eprintln!("Error handling request {} {}: {:?}", method, path, e);
                let response = tiny_http::Response::from_string(format!("Internal Server Error: {}", e))
                    .with_status_code(500);
                let _ = request.respond(response);
            }
        }
    }

    Ok(())
}

/// Read a string from WASM memory given a raw (ptr, len) pair.
fn read_string_from_wasm(store: &mut Store<()>, instance: &Instance, ptr: i32, len: i32) -> wasmtime::Result<String> {
    let memory = get_instance_memory(store, instance)
        .ok_or_else(|| Error::msg("No memory export"))?;
    let data = memory.data(&store);
    let start = ptr as usize;
    let end = start + len as usize;
    if end > data.len() {
        return Err(Error::msg(format!("String read out of bounds: ptr={}, len={}", ptr, len)));
    }
    Ok(String::from_utf8_lossy(&data[start..end]).to_string())
}

fn handle_wasm_request(
    instance: &Instance,
    store: &mut Store<()>,
    method: &str,
    path: &str,
    body: &str,
) -> wasmtime::Result<String> {
    let handle_http = instance
        .get_typed_func::<(i32, i32, i32, i32, i32, i32), (i32, i32)>(&mut *store, "__handle_http")?;

    let memory = get_instance_memory(store, instance)
        .ok_or_else(|| Error::msg("No memory export"))?;
    let heap_global = instance.get_global(&mut *store, "heap_ptr")
        .ok_or_else(|| Error::msg("No heap_ptr export"))?;

    let method_bytes = method.as_bytes();
    let path_bytes = path.as_bytes();
    let body_bytes = body.as_bytes();

    let alloc_raw = |store: &mut Store<()>, bytes: &[u8]| -> wasmtime::Result<(i32, i32)> {
        let heap_ptr = heap_global.get(&mut *store).i32().unwrap_or(0) as usize;
        let len = bytes.len();
        let aligned = (len + 7) & !7;

        let mem_size = memory.data_size(&*store);
        if heap_ptr + len > mem_size {
            let pages_needed = ((heap_ptr + len - mem_size) + 65535) / 65536;
            memory.grow(&mut *store, pages_needed as u64)?;
        }

        let data = memory.data_mut(&mut *store);
        data[heap_ptr..heap_ptr + len].copy_from_slice(bytes);

        let new_heap = (heap_ptr + aligned) as i32;
        heap_global.set(&mut *store, Val::I32(new_heap))?;

        Ok((heap_ptr as i32, len as i32))
    };

    let (m_ptr, m_len) = alloc_raw(store, method_bytes)?;
    let (p_ptr, p_len) = alloc_raw(store, path_bytes)?;
    let (b_ptr, b_len) = alloc_raw(store, body_bytes)?;

    let (resp_ptr, resp_len) = handle_http.call(&mut *store, (m_ptr, m_len, p_ptr, p_len, b_ptr, b_len))?;

    read_string_from_wasm(store, instance, resp_ptr, resp_len)
}

/// Register all JAPL host functions. For serve we provide simplified stubs
/// for process/TCP/etc and full implementations for string functions.
fn register_host_functions(linker: &mut Linker<()>) -> wasmtime::Result<()> {
    // wasi_snapshot_preview1.fd_write -- minimal implementation for println
    linker.func_wrap("wasi_snapshot_preview1", "fd_write",
        |mut caller: Caller<'_, ()>, fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32| -> i32 {
            let memory = match get_caller_memory(&mut caller) {
                Some(m) => m,
                None => return -1,
            };
            let data = memory.data_mut(&mut caller);
            let mut total_written: u32 = 0;

            for i in 0..iovs_len as usize {
                let iov_offset = iovs_ptr as usize + i * 8;
                if iov_offset + 8 > data.len() { break; }
                let buf_ptr = u32::from_le_bytes(data[iov_offset..iov_offset+4].try_into().unwrap()) as usize;
                let buf_len = u32::from_le_bytes(data[iov_offset+4..iov_offset+8].try_into().unwrap()) as usize;
                if buf_ptr + buf_len <= data.len() {
                    let bytes = &data[buf_ptr..buf_ptr + buf_len];
                    if fd == 1 {
                        let _ = std::io::stdout().write_all(bytes);
                    } else if fd == 2 {
                        let _ = std::io::stderr().write_all(bytes);
                    }
                    total_written += buf_len as u32;
                }
            }

            if fd == 1 { let _ = std::io::stdout().flush(); }
            if fd == 2 { let _ = std::io::stderr().flush(); }

            let data = memory.data_mut(&mut caller);
            let nw = nwritten_ptr as usize;
            if nw + 4 <= data.len() {
                data[nw..nw+4].copy_from_slice(&total_written.to_le_bytes());
            }

            0
        })?;

    // japl.println(ptr, len)
    linker.func_wrap("japl", "println", |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
        let mem = get_caller_memory(&mut caller);
        if let Some(mem) = mem {
            let data = mem.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end <= data.len() {
                if let Ok(s) = std::str::from_utf8(&data[start..end]) {
                    println!("{}", s);
                    let _ = std::io::stdout().flush();
                }
            }
        }
    })?;

    // String manipulation functions (fully implemented)
    linker.func_wrap("japl", "char_at", |mut caller: Caller<'_, ()>, str_ptr: i32, index: i32| -> i32 {
        let memory = get_caller_memory(&mut caller).unwrap();
        let data = memory.data(&caller);
        let ptr = str_ptr as usize;
        let len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as usize;
        let idx = index as usize;
        if idx >= len { return -1; }
        data[ptr + 4 + idx] as i32
    })?;

    linker.func_wrap("japl", "substring", |mut caller: Caller<'_, ()>, str_ptr: i32, start: i32, end: i32| -> i32 {
        let memory = get_caller_memory(&mut caller).unwrap();
        let heap_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
        let data = memory.data(&caller);
        let ptr = str_ptr as usize;
        let orig_len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as usize;
        let s = start.max(0) as usize;
        let e = (end as usize).min(orig_len);
        let new_len = if e > s { e - s } else { 0 };
        let heap_ptr = heap_global.get(&mut caller).i32().unwrap() as usize;
        let result_ptr = heap_ptr;
        let mem = memory.data_mut(&mut caller);
        mem[result_ptr..result_ptr+4].copy_from_slice(&(new_len as u32).to_le_bytes());
        if new_len > 0 {
            let src_start = ptr + 4 + s;
            for i in 0..new_len {
                mem[result_ptr + 4 + i] = mem[src_start + i];
            }
        }
        let new_heap = (result_ptr + 4 + new_len) as i32;
        heap_global.set(&mut caller, new_heap.into()).unwrap();
        result_ptr as i32
    })?;

    linker.func_wrap("japl", "string_index_of", |mut caller: Caller<'_, ()>, hay_ptr: i32, needle_ptr: i32| -> i32 {
        let memory = get_caller_memory(&mut caller).unwrap();
        let data = memory.data(&caller);
        let h = hay_ptr as usize;
        let h_len = u32::from_le_bytes(data[h..h+4].try_into().unwrap()) as usize;
        let haystack = &data[h+4..h+4+h_len];
        let n = needle_ptr as usize;
        let n_len = u32::from_le_bytes(data[n..n+4].try_into().unwrap()) as usize;
        let needle = &data[n+4..n+4+n_len];
        if n_len == 0 { return 0; }
        if n_len > h_len { return -1; }
        for i in 0..=(h_len - n_len) {
            if &haystack[i..i+n_len] == needle {
                return i as i32;
            }
        }
        -1
    })?;

    linker.func_wrap("japl", "from_char_code", |mut caller: Caller<'_, ()>, code: i32| -> i32 {
        let memory = get_caller_memory(&mut caller).unwrap();
        let heap_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
        let heap_ptr = heap_global.get(&mut caller).i32().unwrap() as usize;
        let mem = memory.data_mut(&mut caller);
        mem[heap_ptr..heap_ptr+4].copy_from_slice(&1u32.to_le_bytes());
        mem[heap_ptr + 4] = code as u8;
        let new_heap = (heap_ptr + 5) as i32;
        heap_global.set(&mut caller, new_heap.into()).unwrap();
        heap_ptr as i32
    })?;

    linker.func_wrap("japl", "str_length", |mut caller: Caller<'_, ()>, str_ptr: i32| -> i32 {
        let memory = get_caller_memory(&mut caller).unwrap();
        let data = memory.data(&caller);
        let ptr = str_ptr as usize;
        u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as i32
    })?;

    linker.func_wrap("japl", "string_eq", |mut caller: Caller<'_, ()>, a_ptr: i32, b_ptr: i32| -> i32 {
        let memory = get_caller_memory(&mut caller).unwrap();
        let data = memory.data(&caller);
        let a = a_ptr as usize;
        let b = b_ptr as usize;
        let a_len = u32::from_le_bytes(data[a..a+4].try_into().unwrap()) as usize;
        let b_len = u32::from_le_bytes(data[b..b+4].try_into().unwrap()) as usize;
        if a_len != b_len { return 0; }
        if data[a+4..a+4+a_len] == data[b+4..b+4+b_len] { 1 } else { 0 }
    })?;

    linker.func_wrap("japl", "print_bytes", |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
        let memory = get_caller_memory(&mut caller).unwrap();
        let data = &memory.data(&caller)[ptr as usize..(ptr + len) as usize];
        std::io::stdout().write_all(data).ok();
        std::io::stdout().flush().ok();
    })?;

    linker.func_wrap("japl", "bytes_alloc", |mut caller: Caller<'_, ()>, len: i32| -> i32 {
        let heap_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
        let heap_ptr = heap_global.get(&mut caller).i32().unwrap();
        let aligned = (len + 7) & !7;
        heap_global.set(&mut caller, (heap_ptr + aligned).into()).unwrap();
        heap_ptr
    })?;

    // Stubs for functions not needed in HTTP serving mode
    linker.func_wrap("japl", "spawn", |_: i64| -> i64 { -1 })?;
    linker.func_wrap("japl", "spawn_remote", |_: i32, _: i64| -> i64 { -1 })?;
    linker.func_wrap("japl", "send", |_: i64, _: i64| {})?;
    linker.func_wrap("japl", "receive", || -> i64 { -1 })?;
    linker.func_wrap("japl", "self_pid", || -> i64 { 0 })?;
    linker.func_wrap("japl", "llm", |_: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "llm_str", |_: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "llm_structured_str", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "tcp_listen", |_: i32| -> i64 { -1 })?;
    linker.func_wrap("japl", "tcp_accept", |_: i64| -> i64 { -1 })?;
    linker.func_wrap("japl", "tcp_connect", |_: i32, _: i32, _: i32| -> i64 { -1 })?;
    linker.func_wrap("japl", "tcp_read", |_: i64, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "tcp_write", |_: i64, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "tcp_close", |_: i64| {})?;
    linker.func_wrap("japl", "time_now", || -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    })?;
    linker.func_wrap("japl", "time_sleep", |millis: i64| {
        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
    })?;
    linker.func_wrap("japl", "env_get", |_: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "env_get_str", |_: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "env_args_count", || -> i32 { 0 })?;
    linker.func_wrap("japl", "crypto_sha256", |_: i32, _: i32, _: i32| {})?;
    linker.func_wrap("japl", "crypto_random", |_: i32, _: i32| {})?;
    linker.func_wrap("japl", "file_read_str", |_: i32| -> i32 { 0 })?;
    linker.func_wrap("japl", "file_read", |_: i32, _: i32| -> (i32, i32) { (0, 0) })?;
    linker.func_wrap("japl", "file_write", |_: i32, _: i32, _: i32, _: i32| -> i32 { -1 })?;
    linker.func_wrap("japl", "file_exists", |_: i32, _: i32| -> i32 { 0 })?;

    Ok(())
}

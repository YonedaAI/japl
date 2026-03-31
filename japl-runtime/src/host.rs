use std::io::{Read, Write};
use std::sync::mpsc;
use wasmtime::*;

use crate::process::{self, ProcessMessage, ProcessState, Resource, SchedulerCommand};

/// Register JAPL host functions on the linker.
pub fn add_japl_host_functions(linker: &mut Linker<ProcessState>) -> anyhow::Result<()> {
    // japl.spawn(closure_ptr) -> pid
    // closure_ptr is an i64 pointer to a closure struct in linear memory.
    // The runtime calls __process_entry(closure_ptr) in the new process.
    linker.func_wrap("japl", "spawn", |mut caller: Caller<'_, ProcessState>, closure_ptr: i64| -> i64 {
        // Copy the closure struct from the parent's memory so the child can access it.
        // Read the closure data (table_index + captures) from parent memory.
        let mem = caller.get_export("memory").and_then(|e| e.into_memory());
        let closure_bytes = if let Some(mem) = mem {
            let data = mem.data(&caller);
            let ptr = closure_ptr as usize;
            // Read enough bytes for the closure (8 bytes table_idx + up to 256 bytes captures)
            // We'll copy a generous 256 bytes from the closure pointer
            let end = (ptr + 256).min(data.len());
            data[ptr..end].to_vec()
        } else {
            vec![]
        };

        let state = caller.data();
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = state.scheduler_tx.send(SchedulerCommand::SpawnClosure {
            closure_ptr,
            closure_bytes,
            reply: reply_tx,
        });
        // Block until scheduler replies with new PID
        match reply_rx.recv() {
            Ok(pid) => pid as i64,
            Err(_) => -1,
        }
    })?;

    // japl.spawn_remote(node_id, closure_ptr) -> pid
    // Like spawn, but the process is created on a remote node identified by node_id.
    // Returns a remote PID (high 32 bits = node_id, low 32 bits = remote local pid).
    linker.func_wrap("japl", "spawn_remote", |mut caller: Caller<'_, ProcessState>, node_id: i32, closure_ptr: i64| -> i64 {
        let mem = caller.get_export("memory").and_then(|e| e.into_memory());
        let closure_bytes = if let Some(mem) = mem {
            let data = mem.data(&caller);
            let ptr = closure_ptr as usize;
            let end = (ptr + 256).min(data.len());
            data[ptr..end].to_vec()
        } else {
            vec![]
        };

        let state = caller.data();
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = state.scheduler_tx.send(SchedulerCommand::RemoteSpawn {
            node_id: node_id as u32,
            closure_ptr,
            closure_bytes,
            reply: reply_tx,
        });

        // Block until the remote node replies with the new PID
        match reply_rx.recv() {
            Ok(remote_local_pid) => {
                // Encode the remote PID with the node_id in the high bits
                process::make_remote_pid(node_id as u32, remote_local_pid as u32) as i64
            }
            Err(_) => -1,
        }
    })?;

    // japl.send(pid, msg_ptr)
    // msg_ptr is an i64 pointer to a variant struct in linear memory.
    // Read the variant struct (tag + field_count + fields) and send as bytes.
    linker.func_wrap("japl", "send", |mut caller: Caller<'_, ProcessState>, pid: i64, msg_ptr: i64| {
        let msg_bytes = {
            let mem = caller.get_export("memory").and_then(|e| e.into_memory());
            if let Some(mem) = mem {
                let data = mem.data(&caller);
                let ptr = msg_ptr as usize;
                if ptr + 8 <= data.len() {
                    // Read tag (4 bytes) and field_count (4 bytes)
                    let _tag = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap());
                    let field_count = u32::from_le_bytes(data[ptr+4..ptr+8].try_into().unwrap());
                    let total_size = 8 + (field_count as usize) * 8; // tag(4) + count(4) + fields(8 each)
                    let end = (ptr + total_size).min(data.len());
                    data[ptr..end].to_vec()
                } else {
                    // Small value - send as raw i64 bytes
                    msg_ptr.to_le_bytes().to_vec()
                }
            } else {
                msg_ptr.to_le_bytes().to_vec()
            }
        };
        let target_pid = pid as u64;
        let from_pid = caller.data().pid;
        if process::is_local_pid(target_pid) {
            // Local send -- goes through scheduler's local mailbox delivery
            let _ = caller.data().scheduler_tx.send(SchedulerCommand::Send {
                target_pid,
                message_bytes: msg_bytes,
            });
        } else {
            // Remote send -- route through distribution layer via scheduler
            let node_id = process::node_id_from_pid(target_pid);
            let _ = caller.data().scheduler_tx.send(SchedulerCommand::RemoteSend {
                node_id,
                target_pid,
                from_pid,
                message_bytes: msg_bytes,
            });
        }
    })?;

    // japl.receive() -> msg_ptr
    // Blocks until a message is available in the mailbox.
    // Allocates space in receiver's memory and writes the variant bytes.
    linker.func_wrap("japl", "receive", |mut caller: Caller<'_, ProcessState>| -> i64 {
        // Get message bytes from mailbox or channel
        let msg_bytes = {
            let state = caller.data_mut();
            if let Some(msg) = state.mailbox.pop_front() {
                msg
            } else {
                // Block until a message arrives. Using recv() instead of
                // recv_timeout() eliminates timing gaps that caused flaky
                // test failures when messages arrived between timeout cycles.
                loop {
                    match state.receiver.recv() {
                        Ok(ProcessMessage::Deliver(msg)) => break msg,
                        Ok(ProcessMessage::Shutdown) => return -1,
                        Err(mpsc::RecvError) => return -1,
                    }
                }
            }
        };

        // Allocate space in receiver's memory by calling $alloc
        // Allocate in receiver's memory by manipulating heap_ptr directly

        // Direct memory allocation: read heap_ptr, write bytes, advance heap_ptr
        let mem = caller.get_export("memory").and_then(|e| e.into_memory());
        let heap_ptr_global = caller.get_export("heap_ptr").and_then(|e| e.into_global());

        if let (Some(mem), Some(heap_ptr_global)) = (mem, heap_ptr_global) {
            let heap_ptr = heap_ptr_global.get(&mut caller).i32().unwrap_or(0) as usize;
            let size = msg_bytes.len();
            let aligned_size = (size + 7) & !7;

            // Write message bytes into memory at heap_ptr
            let data = mem.data_mut(&mut caller);
            if heap_ptr + size <= data.len() {
                data[heap_ptr..heap_ptr + size].copy_from_slice(&msg_bytes);
            }

            // Advance heap_ptr
            let new_heap = (heap_ptr + aligned_size) as i32;
            let _ = heap_ptr_global.set(&mut caller, Val::I32(new_heap));

            return heap_ptr as i64;
        }

        -1
    })?;

    // japl.self_pid() -> pid
    linker.func_wrap("japl", "self_pid", |caller: Caller<'_, ProcessState>| -> i64 {
        caller.data().pid as i64
    })?;

    // japl.llm(ptr, len) -> (result_ptr, result_len)
    // Reads a prompt string from WASM memory, calls LLM API (or returns mock), writes response back
    linker.func_wrap("japl", "llm", |mut caller: Caller<'_, ProcessState>, ptr: i32, len: i32| -> (i32, i32) {
        // Read prompt string from WASM memory
        let prompt = {
            let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = mem.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end <= data.len() {
                std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
            } else {
                String::new()
            }
        };

        // Call LLM API or return mock
        let response = call_llm_api(&prompt);

        // Write response into WASM memory via heap_ptr
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        let heap_ptr_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
        let heap_ptr = heap_ptr_global.get(&mut caller).i32().unwrap_or(0) as usize;

        let response_bytes = response.as_bytes();
        let result_ptr = heap_ptr;

        let data = mem.data_mut(&mut caller);
        if result_ptr + response_bytes.len() <= data.len() {
            data[result_ptr..result_ptr + response_bytes.len()].copy_from_slice(response_bytes);
        }

        // Advance heap_ptr (aligned)
        let new_heap = ((result_ptr + response_bytes.len() + 7) & !7) as i32;
        let _ = heap_ptr_global.set(&mut caller, Val::I32(new_heap));

        (result_ptr as i32, response_bytes.len() as i32)
    })?;

    // japl.println(ptr, len) — read a UTF-8 string from WASM memory and print it
    linker.func_wrap("japl", "println", |mut caller: Caller<'_, ProcessState>, ptr: i32, len: i32| {
        let mem = caller.get_export("memory")
            .and_then(|e| e.into_memory());
        if let Some(mem) = mem {
            let data = mem.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end <= data.len() {
                if let Ok(s) = std::str::from_utf8(&data[start..end]) {
                    println!("{}", s);
                    // Flush stdout immediately so output appears in order
                    // before any subsequently spawned process writes.
                    let _ = std::io::stdout().flush();
                }
            }
        }
    })?;

    // =========================================================================
    // TCP Functions
    // =========================================================================

    // japl.tcp_listen(port: i32) -> i64 (listener_id, or -1 on error)
    linker.func_wrap("japl", "tcp_listen", |mut caller: Caller<'_, ProcessState>, port: i32| -> i64 {
        use std::net::TcpListener;
        match TcpListener::bind(format!("0.0.0.0:{}", port)) {
            Ok(listener) => {
                listener.set_nonblocking(false).ok();
                let id = caller.data_mut().register_resource(Resource::TcpListener(listener));
                id as i64
            }
            Err(_) => -1,
        }
    })?;

    // japl.tcp_accept(listener_id: i64) -> i64 (conn_id, or -1)
    linker.func_wrap("japl", "tcp_accept", |mut caller: Caller<'_, ProcessState>, listener_id: i64| -> i64 {
        // We need to extract the listener, accept, then put it back and register the stream.
        // Since we can't hold a mutable ref to two things at once, remove-then-reinsert.
        let listener = caller.data_mut().resources.remove(&(listener_id as u64));
        match listener {
            Some(Resource::TcpListener(l)) => {
                let result = l.accept();
                // Put listener back
                caller.data_mut().resources.insert(listener_id as u64, Resource::TcpListener(l));
                match result {
                    Ok((stream, _addr)) => {
                        let id = caller.data_mut().register_resource(Resource::TcpStream(stream));
                        id as i64
                    }
                    Err(_) => -1,
                }
            }
            Some(other) => {
                // Not a listener, put it back
                caller.data_mut().resources.insert(listener_id as u64, other);
                -1
            }
            None => -1,
        }
    })?;

    // japl.tcp_connect(host_ptr: i32, host_len: i32, port: i32) -> i64
    linker.func_wrap("japl", "tcp_connect", |mut caller: Caller<'_, ProcessState>, host_ptr: i32, host_len: i32, port: i32| -> i64 {
        let host = {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let start = host_ptr as usize;
            let end = start + host_len as usize;
            if end > data.len() {
                return -1;
            }
            std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
        };
        match std::net::TcpStream::connect(format!("{}:{}", host, port)) {
            Ok(stream) => {
                let id = caller.data_mut().register_resource(Resource::TcpStream(stream));
                id as i64
            }
            Err(_) => -1,
        }
    })?;

    // japl.tcp_read(conn_id: i64, buf_ptr: i32, buf_len: i32) -> i32 (bytes_read, or -1)
    linker.func_wrap("japl", "tcp_read", |mut caller: Caller<'_, ProcessState>, conn_id: i64, buf_ptr: i32, buf_len: i32| -> i32 {
        // Remove stream to get mutable access without borrow conflicts
        let stream = caller.data_mut().resources.remove(&(conn_id as u64));
        match stream {
            Some(Resource::TcpStream(mut s)) => {
                let mut tmp = vec![0u8; buf_len as usize];
                let result = match s.read(&mut tmp) {
                    Ok(n) => {
                        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                        let mem = memory.data_mut(&mut caller);
                        mem[buf_ptr as usize..buf_ptr as usize + n].copy_from_slice(&tmp[..n]);
                        n as i32
                    }
                    Err(_) => -1,
                };
                // Put stream back
                caller.data_mut().resources.insert(conn_id as u64, Resource::TcpStream(s));
                result
            }
            Some(other) => {
                caller.data_mut().resources.insert(conn_id as u64, other);
                -1
            }
            None => -1,
        }
    })?;

    // japl.tcp_write(conn_id: i64, buf_ptr: i32, buf_len: i32) -> i32 (bytes_written, or -1)
    linker.func_wrap("japl", "tcp_write", |mut caller: Caller<'_, ProcessState>, conn_id: i64, buf_ptr: i32, buf_len: i32| -> i32 {
        let bytes = {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let start = buf_ptr as usize;
            let end = start + buf_len as usize;
            if end > data.len() {
                return -1;
            }
            data[start..end].to_vec()
        };
        let stream = caller.data_mut().resources.remove(&(conn_id as u64));
        match stream {
            Some(Resource::TcpStream(mut s)) => {
                let result = match s.write(&bytes) {
                    Ok(n) => n as i32,
                    Err(_) => -1,
                };
                caller.data_mut().resources.insert(conn_id as u64, Resource::TcpStream(s));
                result
            }
            Some(other) => {
                caller.data_mut().resources.insert(conn_id as u64, other);
                -1
            }
            None => -1,
        }
    })?;

    // japl.tcp_close(resource_id: i64)
    linker.func_wrap("japl", "tcp_close", |mut caller: Caller<'_, ProcessState>, id: i64| {
        caller.data_mut().close_resource(id as u64);
    })?;

    // =========================================================================
    // Time Functions
    // =========================================================================

    // japl.time_now() -> i64 (unix milliseconds)
    linker.func_wrap("japl", "time_now", || -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    })?;

    // japl.time_sleep(millis: i64)
    linker.func_wrap("japl", "time_sleep", |millis: i64| {
        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
    })?;

    // =========================================================================
    // Environment Functions
    // =========================================================================

    // japl.env_get(key_ptr: i32, key_len: i32) -> (i32, i32) (val_ptr, val_len)
    linker.func_wrap("japl", "env_get", |mut caller: Caller<'_, ProcessState>, key_ptr: i32, key_len: i32| -> (i32, i32) {
        let key = {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let start = key_ptr as usize;
            let end = start + key_len as usize;
            if end > data.len() {
                return (0, 0);
            }
            std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
        };

        match std::env::var(&key) {
            Ok(val) => {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let heap_ptr = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
                let ptr = heap_ptr.get(&mut caller).i32().unwrap() as usize;
                let bytes = val.as_bytes();
                let mem = memory.data_mut(&mut caller);
                if ptr + bytes.len() > mem.len() {
                    return (0, 0);
                }
                mem[ptr..ptr + bytes.len()].copy_from_slice(bytes);
                let new_heap = ((ptr + bytes.len() + 7) & !7) as i32;
                let _ = heap_ptr.set(&mut caller, Val::I32(new_heap));
                (ptr as i32, bytes.len() as i32)
            }
            Err(_) => (0, 0),
        }
    })?;

    // japl.env_args_count() -> i32
    linker.func_wrap("japl", "env_args_count", || -> i32 {
        std::env::args().count() as i32
    })?;

    // =========================================================================
    // Crypto Functions
    // =========================================================================

    // japl.crypto_sha256(data_ptr: i32, data_len: i32, out_ptr: i32)
    linker.func_wrap("japl", "crypto_sha256", |mut caller: Caller<'_, ProcessState>, data_ptr: i32, data_len: i32, out_ptr: i32| {
        use sha2::{Sha256, Digest};
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let input = memory.data(&caller)[data_ptr as usize..(data_ptr + data_len) as usize].to_vec();
        let hash = Sha256::digest(&input);
        let mem = memory.data_mut(&mut caller);
        mem[out_ptr as usize..out_ptr as usize + 32].copy_from_slice(&hash);
    })?;

    // japl.crypto_random(buf_ptr: i32, buf_len: i32)
    linker.func_wrap("japl", "crypto_random", |mut caller: Caller<'_, ProcessState>, buf_ptr: i32, buf_len: i32| {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let mem = memory.data_mut(&mut caller);
        let buf = &mut mem[buf_ptr as usize..(buf_ptr + buf_len) as usize];
        rng.fill_bytes(buf);
    })?;

    // =========================================================================
    // File I/O Functions
    // =========================================================================

    // japl.file_read_str(japl_str_ptr: i32) -> i32 (japl_str_ptr with file contents)
    // Takes a JAPL string (length-prefixed), reads the file, returns a new JAPL string
    linker.func_wrap("japl", "file_read_str", |mut caller: Caller<'_, ProcessState>, str_ptr: i32| -> i32 {
        let (path, memory, heap_global) = {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let ptr = str_ptr as usize;
            let len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as usize;
            let path_bytes = &data[ptr+4..ptr+4+len];
            let path = std::str::from_utf8(path_bytes).unwrap_or("").to_string();
            let heap_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
            (path, memory, heap_global)
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                let bytes = contents.as_bytes();
                let heap_ptr = heap_global.get(&mut caller).i32().unwrap() as usize;
                let result_ptr = heap_ptr;

                let mem = memory.data_mut(&mut caller);
                // Write length-prefixed JAPL string
                mem[result_ptr..result_ptr+4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
                mem[result_ptr+4..result_ptr+4+bytes.len()].copy_from_slice(bytes);

                let new_heap = (result_ptr + 4 + bytes.len()) as i32;
                heap_global.set(&mut caller, new_heap.into()).unwrap();

                result_ptr as i32
            }
            Err(e) => {
                eprintln!("file_read_str error: {}: {}", path, e);
                0 // return null pointer on error
            }
        }
    })?;

    // japl.file_read(path_ptr: i32, path_len: i32) -> (i32, i32) (data_ptr, data_len)
    linker.func_wrap("japl", "file_read", |mut caller: Caller<'_, ProcessState>, path_ptr: i32, path_len: i32| -> (i32, i32) {
        let path = {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let start = path_ptr as usize;
            let end = start + path_len as usize;
            if end > data.len() {
                return (0, 0);
            }
            std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
        };

        match std::fs::read(&path) {
            Ok(contents) => {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let heap_ptr = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
                let ptr = heap_ptr.get(&mut caller).i32().unwrap() as usize;
                let mem = memory.data_mut(&mut caller);
                if ptr + contents.len() > mem.len() {
                    return (0, 0);
                }
                mem[ptr..ptr + contents.len()].copy_from_slice(&contents);
                let new_heap = ((ptr + contents.len() + 7) & !7) as i32;
                let _ = heap_ptr.set(&mut caller, Val::I32(new_heap));
                (ptr as i32, contents.len() as i32)
            }
            Err(_) => (0, 0),
        }
    })?;

    // japl.file_write(path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32) -> i32
    linker.func_wrap("japl", "file_write", |mut caller: Caller<'_, ProcessState>, path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = memory.data(&caller);
        let path_start = path_ptr as usize;
        let path_end = path_start + path_len as usize;
        let data_start = data_ptr as usize;
        let data_end = data_start + data_len as usize;
        if path_end > data.len() || data_end > data.len() {
            return -1;
        }
        let path = std::str::from_utf8(&data[path_start..path_end]).unwrap_or("").to_string();
        let contents = data[data_start..data_end].to_vec();

        match std::fs::write(&path, &contents) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    })?;

    // japl.file_exists(path_ptr: i32, path_len: i32) -> i32 (0 or 1)
    linker.func_wrap("japl", "file_exists", |mut caller: Caller<'_, ProcessState>, path_ptr: i32, path_len: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = memory.data(&caller);
        let start = path_ptr as usize;
        let end = start + path_len as usize;
        if end > data.len() {
            return 0;
        }
        let path = std::str::from_utf8(&data[start..end]).unwrap_or("");
        if std::path::Path::new(path).exists() { 1 } else { 0 }
    })?;

    // =========================================================================
    // Bytes Helper Functions
    // =========================================================================

    // japl.bytes_alloc(len: i32) -> i32 (pointer)
    linker.func_wrap("japl", "bytes_alloc", |mut caller: Caller<'_, ProcessState>, len: i32| -> i32 {
        let heap_ptr = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
        let ptr = heap_ptr.get(&mut caller).i32().unwrap();
        let new_heap = ((ptr + len + 7) & !7) as i32;
        let _ = heap_ptr.set(&mut caller, Val::I32(new_heap));
        ptr
    })?;

    // japl.print_bytes(ptr: i32, len: i32)
    linker.func_wrap("japl", "print_bytes", |mut caller: Caller<'_, ProcessState>, ptr: i32, len: i32| {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = &memory.data(&caller)[ptr as usize..(ptr + len) as usize];
        std::io::stdout().write_all(data).ok();
        std::io::stdout().flush().ok();
    })?;

    // =========================================================================
    // String Manipulation Functions
    // =========================================================================

    // japl.char_at(str_ptr: i32, index: i32) -> i32 (char code, or -1 if out of bounds)
    linker.func_wrap("japl", "char_at", |mut caller: Caller<'_, ProcessState>, str_ptr: i32, index: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = memory.data(&caller);
        let ptr = str_ptr as usize;
        let len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as usize;
        let idx = index as usize;
        if idx >= len { return -1; }
        data[ptr + 4 + idx] as i32
    })?;

    // japl.substring(str_ptr: i32, start: i32, end: i32) -> i32 (new string ptr)
    linker.func_wrap("japl", "substring", |mut caller: Caller<'_, ProcessState>, str_ptr: i32, start: i32, end: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let heap_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();

        let data = memory.data(&caller);
        let ptr = str_ptr as usize;
        let orig_len = u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as usize;

        let s = start.max(0) as usize;
        let e = (end as usize).min(orig_len);
        let new_len = if e > s { e - s } else { 0 };

        // Allocate new string
        let heap_ptr = heap_global.get(&mut caller).i32().unwrap() as usize;
        let result_ptr = heap_ptr;

        let mem = memory.data_mut(&mut caller);
        // Write length
        mem[result_ptr..result_ptr+4].copy_from_slice(&(new_len as u32).to_le_bytes());
        // Copy bytes
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

    // japl.string_index_of(haystack_ptr: i32, needle_ptr: i32) -> i32 (index, or -1)
    linker.func_wrap("japl", "string_index_of", |mut caller: Caller<'_, ProcessState>, hay_ptr: i32, needle_ptr: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = memory.data(&caller);

        let h_ptr = hay_ptr as usize;
        let h_len = u32::from_le_bytes(data[h_ptr..h_ptr+4].try_into().unwrap()) as usize;
        let haystack = &data[h_ptr+4..h_ptr+4+h_len];

        let n_ptr = needle_ptr as usize;
        let n_len = u32::from_le_bytes(data[n_ptr..n_ptr+4].try_into().unwrap()) as usize;
        let needle = &data[n_ptr+4..n_ptr+4+n_len];

        if n_len == 0 { return 0; }
        if n_len > h_len { return -1; }

        for i in 0..=(h_len - n_len) {
            if &haystack[i..i+n_len] == needle {
                return i as i32;
            }
        }
        -1
    })?;

    // japl.from_char_code(code: i32) -> i32 (new 1-char string ptr)
    linker.func_wrap("japl", "from_char_code", |mut caller: Caller<'_, ProcessState>, code: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let heap_global = caller.get_export("heap_ptr").unwrap().into_global().unwrap();
        let heap_ptr = heap_global.get(&mut caller).i32().unwrap() as usize;

        let mem = memory.data_mut(&mut caller);
        mem[heap_ptr..heap_ptr+4].copy_from_slice(&1u32.to_le_bytes());
        mem[heap_ptr + 4] = code as u8;

        let new_heap = (heap_ptr + 5) as i32;
        heap_global.set(&mut caller, new_heap.into()).unwrap();

        heap_ptr as i32
    })?;

    // japl.str_length(str_ptr: i32) -> i32
    linker.func_wrap("japl", "str_length", |mut caller: Caller<'_, ProcessState>, str_ptr: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = memory.data(&caller);
        let ptr = str_ptr as usize;
        u32::from_le_bytes(data[ptr..ptr+4].try_into().unwrap()) as i32
    })?;

    // japl.string_eq(a_ptr: i32, b_ptr: i32) -> i32 (0 or 1)
    linker.func_wrap("japl", "string_eq", |mut caller: Caller<'_, ProcessState>, a_ptr: i32, b_ptr: i32| -> i32 {
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = memory.data(&caller);
        let a = a_ptr as usize;
        let b = b_ptr as usize;
        let a_len = u32::from_le_bytes(data[a..a+4].try_into().unwrap()) as usize;
        let b_len = u32::from_le_bytes(data[b..b+4].try_into().unwrap()) as usize;
        if a_len != b_len { return 0; }
        if data[a+4..a+4+a_len] == data[b+4..b+4+b_len] { 1 } else { 0 }
    })?;

    Ok(())
}

/// Call an LLM API. Checks for ANTHROPIC_API_KEY or OPENAI_API_KEY in environment.
/// Falls back to a mock response if no key is set.
fn call_llm_api(prompt: &str) -> String {
    // Try Anthropic Claude API first
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        match call_anthropic_api(&api_key, prompt) {
            Ok(response) => return response,
            Err(e) => {
                eprintln!("[JAPL LLM] Anthropic API error: {}", e);
            }
        }
    }

    // Try OpenAI API
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        match call_openai_api(&api_key, prompt) {
            Ok(response) => return response,
            Err(e) => {
                eprintln!("[JAPL LLM] OpenAI API error: {}", e);
            }
        }
    }

    // Mock fallback
    format!("[LLM mock] Received prompt: {}", prompt)
}

fn call_anthropic_api(api_key: &str, prompt: &str) -> Result<String, String> {
    let body = format!(
        r#"{{"model":"claude-sonnet-4-20250514","max_tokens":1024,"messages":[{{"role":"user","content":"{}"}}]}}"#,
        prompt.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let resp_str = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{}", e))?
        .into_string()
        .map_err(|e| format!("Read error: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&resp_str)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    // Extract text from response
    if let Some(content) = json["content"].as_array() {
        for item in content {
            if let Some(text) = item["text"].as_str() {
                return Ok(text.to_string());
            }
        }
    }

    Err("Unexpected response format".to_string())
}

fn call_openai_api(api_key: &str, prompt: &str) -> Result<String, String> {
    let body = format!(
        r#"{{"model":"gpt-4o-mini","messages":[{{"role":"user","content":"{}"}}]}}"#,
        prompt.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let resp_str = ureq::post("https://api.openai.com/v1/chat/completions")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("content-type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{}", e))?
        .into_string()
        .map_err(|e| format!("Read error: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&resp_str)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    if let Some(choices) = json["choices"].as_array() {
        for choice in choices {
            if let Some(text) = choice["message"]["content"].as_str() {
                return Ok(text.to_string());
            }
        }
    }

    Err("Unexpected response format".to_string())
}

use std::sync::mpsc;
use std::time::Duration;
use wasmtime::*;

use crate::process::{ProcessMessage, ProcessState, SchedulerCommand};

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
        let _ = caller.data().scheduler_tx.send(SchedulerCommand::Send {
            target_pid: pid as u64,
            message_bytes: msg_bytes,
        });
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
                loop {
                    match state.receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(ProcessMessage::Deliver(msg)) => break msg,
                        Ok(ProcessMessage::Shutdown) => return -1,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return -1,
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
                }
            }
        }
    })?;

    Ok(())
}

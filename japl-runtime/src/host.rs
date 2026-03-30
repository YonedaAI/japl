use std::sync::mpsc;
use std::time::Duration;
use wasmtime::*;

use crate::process::{ProcessMessage, ProcessState, SchedulerCommand};

/// Register JAPL host functions on the linker.
pub fn add_japl_host_functions(linker: &mut Linker<ProcessState>) -> anyhow::Result<()> {
    // japl.spawn(func_idx) -> pid
    // For now func_idx is ignored; we always call "_start".
    // A real implementation would look up exported functions by index.
    linker.func_wrap("japl", "spawn", |caller: Caller<'_, ProcessState>, _func_idx: i64| -> i64 {
        let state = caller.data();
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = state.scheduler_tx.send(SchedulerCommand::Spawn {
            func_name: "_start".to_string(),
            reply: reply_tx,
        });
        // Block until scheduler replies with new PID
        match reply_rx.recv() {
            Ok(pid) => pid as i64,
            Err(_) => -1,
        }
    })?;

    // japl.send(pid, msg)
    linker.func_wrap("japl", "send", |caller: Caller<'_, ProcessState>, pid: i64, msg: i64| {
        let _ = caller.data().scheduler_tx.send(SchedulerCommand::Send {
            target_pid: pid as u64,
            message: msg,
        });
    })?;

    // japl.receive() -> msg
    // Blocks until a message is available in the mailbox.
    linker.func_wrap("japl", "receive", |mut caller: Caller<'_, ProcessState>| -> i64 {
        let state = caller.data_mut();
        // Check mailbox first
        if let Some(msg) = state.mailbox.pop_front() {
            return msg;
        }
        // Block on channel
        loop {
            match state.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(ProcessMessage::Deliver(msg)) => {
                    return msg;
                }
                Ok(ProcessMessage::Shutdown) => {
                    // Returning a sentinel; the WASM code should handle this
                    return -1;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return -1,
            }
        }
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

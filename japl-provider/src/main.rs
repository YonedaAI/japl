use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// A logical process: just a mailbox and metadata.
struct Process {
    #[allow(dead_code)]
    pid: u64,
    mailbox: Vec<Vec<u8>>,
    notify: Arc<Notify>,
}

/// Shared state for all processes managed by this provider.
struct ProcessTable {
    processes: HashMap<u64, Process>,
    next_pid: u64,
}

impl ProcessTable {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
            next_pid: 1,
        }
    }

    fn spawn(&mut self, _closure_data: Vec<u8>) -> u64 {
        let pid = self.next_pid;
        self.next_pid += 1;
        self.processes.insert(
            pid,
            Process {
                pid,
                mailbox: Vec::new(),
                notify: Arc::new(Notify::new()),
            },
        );
        pid
    }

    fn send(&mut self, pid: u64, message: Vec<u8>) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.mailbox.push(message);
            proc.notify.notify_one();
            true
        } else {
            false
        }
    }

    fn try_receive(&mut self, pid: u64) -> Option<Vec<u8>> {
        if let Some(proc) = self.processes.get_mut(&pid) {
            if !proc.mailbox.is_empty() {
                Some(proc.mailbox.remove(0))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_notify(&self, pid: u64) -> Option<Arc<Notify>> {
        self.processes.get(&pid).map(|p| p.notify.clone())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SpawnRequest {
    closure_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SpawnResponse {
    pid: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendRequest {
    message: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReceiveResponse {
    message: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SelfPidResponse {
    pid: u64,
}

type SharedTable = Arc<Mutex<ProcessTable>>;

async fn handle_spawn(table: &SharedTable, payload: &[u8]) -> Vec<u8> {
    let req: SpawnRequest = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("spawn: bad payload: {e}");
            return serde_json::to_vec(&SpawnResponse { pid: 0 }).unwrap();
        }
    };
    let pid = table.lock().await.spawn(req.closure_data);
    println!("  spawned process pid={pid}");
    serde_json::to_vec(&SpawnResponse { pid }).unwrap()
}

async fn handle_send(table: &SharedTable, pid: u64, payload: &[u8]) -> Vec<u8> {
    let req: SendRequest = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("send: bad payload: {e}");
            return b"err".to_vec();
        }
    };
    let ok = table.lock().await.send(pid, req.message);
    if ok {
        println!("  sent message to pid={pid}");
        b"ok".to_vec()
    } else {
        eprintln!("  send: no such process pid={pid}");
        b"err".to_vec()
    }
}

async fn handle_receive(table: &SharedTable, pid: u64) -> Vec<u8> {
    // Try to receive immediately; if empty, wait for notification then retry.
    loop {
        {
            let mut t = table.lock().await;
            if let Some(msg) = t.try_receive(pid) {
                println!("  receive from pid={pid}: {} bytes", msg.len());
                return serde_json::to_vec(&ReceiveResponse { message: msg }).unwrap();
            }
        }
        // Wait for a message to arrive.
        let notify = {
            let t = table.lock().await;
            match t.get_notify(pid) {
                Some(n) => n,
                None => {
                    eprintln!("  receive: no such process pid={pid}");
                    return b"err".to_vec();
                }
            }
        };
        notify.notified().await;
    }
}

async fn run_self_test(_table: &SharedTable, client: &Client) -> Result<()> {
    println!("\n--- self-test ---");

    // Spawn a process via NATS
    let spawn_req = serde_json::to_vec(&SpawnRequest {
        closure_data: vec![1, 2, 3],
    })?;
    let resp = client
        .request("japl.runtime.spawn", spawn_req.into())
        .await?;
    let spawn_resp: SpawnResponse = serde_json::from_slice(&resp.payload)?;
    println!("  test: spawned pid={}", spawn_resp.pid);

    // Send a message to that process
    let send_req = serde_json::to_vec(&SendRequest {
        message: b"hello from test".to_vec(),
    })?;
    let resp = client
        .request(
            format!("japl.runtime.send.{}", spawn_resp.pid),
            send_req.into(),
        )
        .await?;
    println!("  test: send result={}", String::from_utf8_lossy(&resp.payload));

    // Receive the message
    let resp = client
        .request(
            format!("japl.runtime.receive.{}", spawn_resp.pid),
            "{}".into(),
        )
        .await?;
    let recv_resp: ReceiveResponse = serde_json::from_slice(&resp.payload)?;
    let msg = String::from_utf8_lossy(&recv_resp.message);
    println!("  test: received message=\"{msg}\"");

    assert_eq!(
        recv_resp.message,
        b"hello from test",
        "self-test: message mismatch"
    );
    println!("--- self-test passed ---\n");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    println!("Connecting to NATS at {nats_url}...");

    let client = async_nats::connect(&nats_url).await?;
    println!("JAPL Runtime Provider connected to NATS");

    let table: SharedTable = Arc::new(Mutex::new(ProcessTable::new()));

    // Subscribe to all japl.runtime.> subjects
    let mut sub = client.subscribe("japl.runtime.>").await?;

    // Run self-test in background after subscriptions are active
    let test_client = client.clone();
    let test_table = table.clone();
    tokio::spawn(async move {
        // Small delay to ensure subscription is processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Err(e) = run_self_test(&test_table, &test_client).await {
            eprintln!("self-test failed: {e}");
        }
    });

    println!("Listening on japl.runtime.>");

    while let Some(msg) = sub.next().await {
        let subject = msg.subject.to_string();
        let reply = msg.reply.clone();
        let table = table.clone();
        let client = client.clone();

        tokio::spawn(async move {
            let response = if subject == "japl.runtime.spawn" {
                handle_spawn(&table, &msg.payload).await
            } else if subject.starts_with("japl.runtime.send.") {
                let pid_str = subject.strip_prefix("japl.runtime.send.").unwrap();
                match pid_str.parse::<u64>() {
                    Ok(pid) => handle_send(&table, pid, &msg.payload).await,
                    Err(_) => b"err: invalid pid".to_vec(),
                }
            } else if subject.starts_with("japl.runtime.receive.") {
                let pid_str = subject.strip_prefix("japl.runtime.receive.").unwrap();
                match pid_str.parse::<u64>() {
                    Ok(pid) => handle_receive(&table, pid).await,
                    Err(_) => b"err: invalid pid".to_vec(),
                }
            } else if subject == "japl.runtime.self-pid" {
                // In a real system, this would be context-dependent.
                // For now, return 0 as a placeholder.
                serde_json::to_vec(&SelfPidResponse { pid: 0 }).unwrap()
            } else {
                eprintln!("unknown subject: {subject}");
                b"err: unknown subject".to_vec()
            };

            if let Some(reply_to) = reply {
                if let Err(e) = client.publish(reply_to, response.into()).await {
                    eprintln!("failed to reply: {e}");
                }
            }
        });
    }

    Ok(())
}

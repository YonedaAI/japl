/// TCP distribution layer for cross-node communication.

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::process::{ProcessId, SchedulerCommand};
use crate::wire::{self, WireMessage};

/// Global counter for assigning numeric node IDs to peer connections.
static NEXT_NODE_ID: AtomicU32 = AtomicU32::new(1); // 0 is reserved for "local"

#[allow(dead_code)]
pub struct DistributionLayer {
    node_name: String,
    cookie: String,
    connections: Arc<Mutex<HashMap<String, TcpStream>>>,
    /// Map from numeric node_id -> node_name (for PID routing)
    node_id_to_name: Arc<Mutex<HashMap<u32, String>>>,
    /// Map from node_name -> numeric node_id
    name_to_node_id: Arc<Mutex<HashMap<String, u32>>>,
    scheduler_tx: mpsc::Sender<SchedulerCommand>,
    /// Pending remote spawn requests: request_id -> oneshot reply sender
    pending_spawns: Arc<Mutex<HashMap<u64, mpsc::Sender<ProcessId>>>>,
}

#[allow(dead_code)]
impl DistributionLayer {
    pub fn new(
        node_name: String,
        cookie: String,
        scheduler_tx: mpsc::Sender<SchedulerCommand>,
    ) -> Self {
        Self {
            node_name,
            cookie,
            connections: Arc::new(Mutex::new(HashMap::new())),
            node_id_to_name: Arc::new(Mutex::new(HashMap::new())),
            name_to_node_id: Arc::new(Mutex::new(HashMap::new())),
            scheduler_tx,
            pending_spawns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start listening for incoming connections on `addr` (e.g. ":9000" or "0.0.0.0:9000").
    pub fn listen(&self, addr: &str) -> anyhow::Result<()> {
        // Normalize ":PORT" to "0.0.0.0:PORT"
        let bind_addr = if addr.starts_with(':') {
            format!("0.0.0.0{}", addr)
        } else {
            addr.to_string()
        };

        let listener = TcpListener::bind(&bind_addr)?;
        println!("[{}] Listening on {}", self.node_name, bind_addr);

        let connections = self.connections.clone();
        let scheduler_tx = self.scheduler_tx.clone();
        let my_name = self.node_name.clone();
        let my_cookie = self.cookie.clone();
        let node_id_to_name = self.node_id_to_name.clone();
        let name_to_node_id = self.name_to_node_id.clone();
        let pending_spawns = self.pending_spawns.clone();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        // Read handshake from remote
                        let frame = match wire::read_frame(&mut stream) {
                            Ok(f) => f,
                            Err(e) => {
                                eprintln!("[{}] Failed to read handshake: {}", my_name, e);
                                continue;
                            }
                        };
                        let msg = match wire::decode(&frame) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("[{}] Bad handshake decode: {}", my_name, e);
                                continue;
                            }
                        };

                        match msg {
                            WireMessage::Handshake { node_name, cookie } => {
                                if cookie != my_cookie {
                                    eprintln!(
                                        "[{}] Cookie mismatch from {}, rejecting",
                                        my_name, node_name
                                    );
                                    continue;
                                }

                                // Send HandshakeOk
                                let reply = WireMessage::HandshakeOk {
                                    node_name: my_name.clone(),
                                };
                                if let Err(e) = wire::write_msg(&mut stream, &reply) {
                                    eprintln!(
                                        "[{}] Failed to send handshake ok: {}",
                                        my_name, e
                                    );
                                    continue;
                                }

                                // Assign a numeric node ID to this peer
                                let nid = NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst);
                                node_id_to_name.lock().unwrap().insert(nid, node_name.clone());
                                name_to_node_id.lock().unwrap().insert(node_name.clone(), nid);

                                println!(
                                    "[{}] Accepted connection from node '{}' (node_id={})",
                                    my_name, node_name, nid
                                );

                                // Store the connection (clone the stream for the reader thread)
                                let reader_stream = match stream.try_clone() {
                                    Ok(s) => s,
                                    Err(e) => {
                                        eprintln!("[{}] Clone stream error: {}", my_name, e);
                                        continue;
                                    }
                                };
                                connections
                                    .lock()
                                    .unwrap()
                                    .insert(node_name.clone(), stream);

                                // Spawn reader thread for this connection
                                let sched_tx = scheduler_tx.clone();
                                let peer_name = node_name.clone();
                                let conns = connections.clone();
                                let local_name = my_name.clone();
                                let ps = pending_spawns.clone();
                                std::thread::spawn(move || {
                                    Self::reader_loop(
                                        reader_stream,
                                        &peer_name,
                                        &local_name,
                                        sched_tx,
                                        conns,
                                        ps,
                                    );
                                });
                            }
                            other => {
                                eprintln!(
                                    "[{}] Expected Handshake, got {:?}",
                                    my_name, other
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}] Accept error: {}", my_name, e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Connect to a remote node at `addr` (e.g. "localhost:9000").
    pub fn connect(&self, addr: &str) -> anyhow::Result<()> {
        let mut stream = TcpStream::connect(addr)?;

        // Send handshake
        let handshake = WireMessage::Handshake {
            node_name: self.node_name.clone(),
            cookie: self.cookie.clone(),
        };
        wire::write_msg(&mut stream, &handshake)?;

        // Read handshake ok
        let frame = wire::read_frame(&mut stream)?;
        let msg = wire::decode(&frame)?;

        match msg {
            WireMessage::HandshakeOk { node_name } => {
                // Assign a numeric node ID to this peer
                let nid = NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst);
                self.node_id_to_name.lock().unwrap().insert(nid, node_name.clone());
                self.name_to_node_id.lock().unwrap().insert(node_name.clone(), nid);

                println!(
                    "[{}] Connected to node '{}' at {} (node_id={})",
                    self.node_name, node_name, addr, nid
                );

                let reader_stream = stream.try_clone()?;
                self.connections
                    .lock()
                    .unwrap()
                    .insert(node_name.clone(), stream);

                // Spawn reader thread
                let sched_tx = self.scheduler_tx.clone();
                let peer_name = node_name.clone();
                let conns = self.connections.clone();
                let local_name = self.node_name.clone();
                let ps = self.pending_spawns.clone();
                std::thread::spawn(move || {
                    Self::reader_loop(reader_stream, &peer_name, &local_name, sched_tx, conns, ps);
                });
            }
            other => {
                anyhow::bail!(
                    "[{}] Expected HandshakeOk, got {:?}",
                    self.node_name,
                    other
                );
            }
        }

        Ok(())
    }

    /// Send a message to a process on a remote node (by node_id).
    pub fn remote_send_by_id(
        &self,
        node_id: u32,
        to_pid: ProcessId,
        from_pid: ProcessId,
        msg_bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        let node_name = {
            let map = self.node_id_to_name.lock().unwrap();
            map.get(&node_id).cloned()
        };
        match node_name {
            Some(name) => {
                let mut connections = self.connections.lock().unwrap();
                if let Some(stream) = connections.get_mut(&name) {
                    let wire_msg = WireMessage::Send {
                        to_pid,
                        from_pid,
                        msg_bytes,
                    };
                    wire::write_msg(stream, &wire_msg)?;
                } else {
                    anyhow::bail!("no connection to node '{}' (node_id={})", name, node_id);
                }
            }
            None => {
                anyhow::bail!("unknown node_id {}", node_id);
            }
        }
        Ok(())
    }

    /// Send a message to a process on a remote node (by node name).
    pub fn remote_send(
        &self,
        node: &str,
        to_pid: ProcessId,
        from_pid: ProcessId,
        msg_bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        let mut connections = self.connections.lock().unwrap();
        if let Some(stream) = connections.get_mut(node) {
            let wire_msg = WireMessage::Send {
                to_pid,
                from_pid,
                msg_bytes,
            };
            wire::write_msg(stream, &wire_msg)?;
        } else {
            anyhow::bail!("no connection to node '{}'", node);
        }
        Ok(())
    }

    /// Request a remote node to spawn a process. Returns the remote PID
    /// (with the node's ID encoded in the high 32 bits).
    pub fn remote_spawn(
        &self,
        node_id: u32,
        closure_ptr: i64,
        closure_bytes: Vec<u8>,
        reply: mpsc::Sender<ProcessId>,
    ) -> anyhow::Result<()> {
        use std::sync::atomic::AtomicU64;
        static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

        let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst);

        // Store the reply channel for when SpawnResponse comes back
        self.pending_spawns.lock().unwrap().insert(request_id, reply);

        let node_name = {
            let map = self.node_id_to_name.lock().unwrap();
            map.get(&node_id).cloned()
        };

        match node_name {
            Some(name) => {
                let mut connections = self.connections.lock().unwrap();
                if let Some(stream) = connections.get_mut(&name) {
                    let wire_msg = WireMessage::SpawnRequest {
                        request_id,
                        closure_ptr,
                        closure_bytes,
                    };
                    wire::write_msg(stream, &wire_msg)?;
                } else {
                    self.pending_spawns.lock().unwrap().remove(&request_id);
                    anyhow::bail!("no connection to node '{}' (node_id={})", name, node_id);
                }
            }
            None => {
                self.pending_spawns.lock().unwrap().remove(&request_id);
                anyhow::bail!("unknown node_id {}", node_id);
            }
        }
        Ok(())
    }

    /// Look up the node_id for a given node name.
    pub fn node_id_for_name(&self, name: &str) -> Option<u32> {
        self.name_to_node_id.lock().unwrap().get(name).copied()
    }

    /// Look up the node name for a given node_id.
    pub fn node_name_for_id(&self, node_id: u32) -> Option<String> {
        self.node_id_to_name.lock().unwrap().get(&node_id).cloned()
    }

    /// Check if a node is connected.
    pub fn is_connected(&self, node: &str) -> bool {
        self.connections.lock().unwrap().contains_key(node)
    }

    /// List connected node names.
    pub fn connected_nodes(&self) -> Vec<String> {
        self.connections.lock().unwrap().keys().cloned().collect()
    }

    /// Background reader loop for a single TCP connection.
    fn reader_loop(
        mut stream: TcpStream,
        peer_name: &str,
        local_name: &str,
        scheduler_tx: mpsc::Sender<SchedulerCommand>,
        connections: Arc<Mutex<HashMap<String, TcpStream>>>,
        pending_spawns: Arc<Mutex<HashMap<u64, mpsc::Sender<ProcessId>>>>,
    ) {
        loop {
            let frame = match wire::read_frame(&mut stream) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!(
                        "[{}] Connection to '{}' lost: {}",
                        local_name, peer_name, e
                    );
                    connections.lock().unwrap().remove(peer_name);
                    return;
                }
            };

            let msg = match wire::decode(&frame) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "[{}] Bad message from '{}': {}",
                        local_name, peer_name, e
                    );
                    continue;
                }
            };

            match msg {
                WireMessage::Send {
                    to_pid, msg_bytes, ..
                } => {
                    // Extract the local PID (low 32 bits) for delivery
                    let local_pid = crate::process::local_id_from_pid(to_pid) as u64;
                    let _ = scheduler_tx.send(SchedulerCommand::Send {
                        target_pid: local_pid,
                        message_bytes: msg_bytes,
                    });
                }
                WireMessage::SpawnRequest {
                    request_id,
                    closure_ptr,
                    closure_bytes,
                } => {
                    // Remote node wants us to spawn a process locally
                    println!(
                        "[{}] SpawnRequest from '{}' (request_id={})",
                        local_name, peer_name, request_id
                    );
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = scheduler_tx.send(SchedulerCommand::SpawnClosure {
                        closure_ptr,
                        closure_bytes,
                        reply: reply_tx,
                    });
                    // Wait for the local PID, then send SpawnResponse back
                    let conns = connections.clone();
                    let pn = peer_name.to_string();
                    let ln = local_name.to_string();
                    std::thread::spawn(move || {
                        match reply_rx.recv() {
                            Ok(local_pid) => {
                                let resp = WireMessage::SpawnResponse {
                                    request_id,
                                    pid: local_pid,
                                };
                                if let Some(write_stream) =
                                    conns.lock().unwrap().get_mut(&pn)
                                {
                                    if let Err(e) = wire::write_msg(write_stream, &resp) {
                                        eprintln!(
                                            "[{}] Failed to send SpawnResponse to '{}': {}",
                                            ln, pn, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "[{}] SpawnClosure reply failed: {}",
                                    ln, e
                                );
                            }
                        }
                    });
                }
                WireMessage::SpawnResponse { request_id, pid } => {
                    // We got back a PID from a remote spawn request
                    if let Some(reply_tx) = pending_spawns.lock().unwrap().remove(&request_id) {
                        let _ = reply_tx.send(pid);
                    } else {
                        eprintln!(
                            "[{}] SpawnResponse for unknown request_id {}",
                            local_name, request_id
                        );
                    }
                }
                WireMessage::Ping => {
                    // Respond with Pong (need write access, get from connections)
                    if let Some(write_stream) =
                        connections.lock().unwrap().get_mut(peer_name)
                    {
                        let _ = wire::write_msg(write_stream, &WireMessage::Pong);
                    }
                }
                WireMessage::Pong => {
                    // Heartbeat acknowledged, nothing to do
                }
                other => {
                    eprintln!(
                        "[{}] Unexpected message from '{}': {:?}",
                        local_name, peer_name, other
                    );
                }
            }
        }
    }
}

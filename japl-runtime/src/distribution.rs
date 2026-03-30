/// TCP distribution layer for cross-node communication.

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::process::{ProcessId, SchedulerCommand};
use crate::wire::{self, WireMessage};

#[allow(dead_code)]
pub struct DistributionLayer {
    node_name: String,
    cookie: String,
    connections: Arc<Mutex<HashMap<String, TcpStream>>>,
    scheduler_tx: mpsc::Sender<SchedulerCommand>,
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
            scheduler_tx,
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

                                println!(
                                    "[{}] Accepted connection from node '{}'",
                                    my_name, node_name
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
                                std::thread::spawn(move || {
                                    Self::reader_loop(
                                        reader_stream,
                                        &peer_name,
                                        &local_name,
                                        sched_tx,
                                        conns,
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
                println!(
                    "[{}] Connected to node '{}' at {}",
                    self.node_name, node_name, addr
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
                std::thread::spawn(move || {
                    Self::reader_loop(reader_stream, &peer_name, &local_name, sched_tx, conns);
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

    /// Send a message to a process on a remote node.
    pub fn remote_send(
        &self,
        node: &str,
        to_pid: ProcessId,
        from_pid: ProcessId,
        msg: i64,
    ) -> anyhow::Result<()> {
        let mut connections = self.connections.lock().unwrap();
        if let Some(stream) = connections.get_mut(node) {
            let wire_msg = WireMessage::Send {
                to_pid,
                from_pid,
                msg,
            };
            wire::write_msg(stream, &wire_msg)?;
        } else {
            anyhow::bail!("no connection to node '{}'", node);
        }
        Ok(())
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
                    to_pid, msg, ..
                } => {
                    // Deliver to local process via scheduler
                    let _ = scheduler_tx.send(SchedulerCommand::Send {
                        target_pid: to_pid,
                        message: msg,
                    });
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

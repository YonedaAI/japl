// STATUS: Experimental / Development Infrastructure
// The primary distributed path is wasmCloud (japl deploy).
// This custom TCP layer is retained for development, testing,
// and as a reference implementation.

// =========================================================================
// JAPL Distribution Layer
// =========================================================================
//
// Provides inter-node communication for JAPL clusters. Each node has a
// durable identity derived from hostname + port + an optional configured
// name, so node IDs survive process restarts.
//
// Nodes discover each other via explicit `connect_to` calls. Once
// connected and authenticated (cookie-based handshake), nodes exchange
// wire frames for remote message delivery, process spawning, and
// health monitoring.
//
// Connection health is monitored via periodic PING/PONG heartbeats.
// If MAX_MISSED_HEARTBEATS consecutive heartbeats go unanswered a
// connection is marked unhealthy and the peer is removed.
//
// =========================================================================

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::wire::{
    self, WireFrame, WIRE_PROTOCOL_VERSION,
    MSG_SEND, MSG_SPAWN, MSG_EXIT, MSG_PING, MSG_PONG,
};

// ---- Configuration constants --------------------------------------------

/// Interval between heartbeat PINGs sent to each peer.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// Number of consecutive missed heartbeats before a connection is
/// considered unhealthy and dropped.
const MAX_MISSED_HEARTBEATS: u32 = 3;

/// Default cluster cookie used when none is configured.
const DEFAULT_COOKIE: &str = "japl_default_cookie";

// =========================================================================
// NodeId — durable, deterministic identity
// =========================================================================

/// Generate a durable node ID from hostname, port, and an optional name.
/// The result is a human-readable string of the form `name@host:port` that
/// is deterministic across restarts with the same configuration.
pub fn generate_node_id(host: &str, port: u16, name: Option<&str>) -> String {
    let node_name = name.unwrap_or("japl");
    format!("{}@{}:{}", node_name, host, port)
}

// =========================================================================
// PeerState — per-connection bookkeeping
// =========================================================================

/// Health state of a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionHealth {
    Healthy,
    Unhealthy,
}

/// Tracks a connected peer.
#[derive(Debug)]
struct PeerState {
    /// The peer's durable node ID (received during handshake).
    node_id: String,
    /// Consecutive heartbeats that went unanswered.
    missed_heartbeats: u32,
    /// When the last PONG was received.
    last_pong: Instant,
    /// Current health assessment.
    health: ConnectionHealth,
}

impl PeerState {
    fn new(node_id: String) -> Self {
        Self {
            node_id,
            missed_heartbeats: 0,
            last_pong: Instant::now(),
            health: ConnectionHealth::Healthy,
        }
    }

    /// Record that a PING was sent. If too many go unanswered, mark unhealthy.
    fn record_ping_sent(&mut self) {
        self.missed_heartbeats += 1;
        if self.missed_heartbeats >= MAX_MISSED_HEARTBEATS {
            if self.health != ConnectionHealth::Unhealthy {
                eprintln!(
                    "[dist] Connection to {} marked UNHEALTHY ({} missed heartbeats)",
                    self.node_id, self.missed_heartbeats
                );
                self.health = ConnectionHealth::Unhealthy;
            }
        }
    }

    /// Record that a PONG was received — resets missed counter and marks healthy.
    fn record_pong_received(&mut self) {
        if self.health == ConnectionHealth::Unhealthy {
            eprintln!(
                "[dist] Connection to {} recovered to HEALTHY",
                self.node_id
            );
        }
        self.missed_heartbeats = 0;
        self.last_pong = Instant::now();
        self.health = ConnectionHealth::Healthy;
    }
}

// =========================================================================
// DistributionNode — the main distribution handle
// =========================================================================

/// A JAPL distribution node that can connect to peers and exchange
/// messages over the wire protocol.
#[allow(dead_code)]
pub struct DistributionNode {
    /// This node's durable identity.
    node_id: String,
    /// Cluster cookie for authentication.
    cookie: String,
    /// Port this node listens on (0 if not listening).
    port: u16,
    /// Connected peers keyed by their node_id.
    peers: Arc<Mutex<HashMap<String, PeerState>>>,
    /// TCP streams to peers, keyed by node_id.
    streams: Arc<Mutex<HashMap<String, TcpStream>>>,
}

impl DistributionNode {
    /// Create a new distribution node with the given identity parameters.
    /// The node ID is generated deterministically from hostname + port + name,
    /// so it survives restarts with the same configuration.
    pub fn new(host: &str, port: u16, name: Option<&str>, cookie: Option<&str>) -> Self {
        let node_id = generate_node_id(host, port, name);
        let cookie = cookie.unwrap_or(DEFAULT_COOKIE).to_string();
        eprintln!("[dist] Node ID: {} (protocol v{})", node_id, WIRE_PROTOCOL_VERSION);
        Self {
            node_id,
            cookie,
            port,
            peers: Arc::new(Mutex::new(HashMap::new())),
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Return this node's durable ID.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Start listening for incoming peer connections on the configured port.
    /// Spawns a background thread that accepts connections and performs the
    /// handshake.
    #[allow(dead_code)]
    pub fn listen(&self) -> std::io::Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)?;
        eprintln!("[dist] Listening on {}", addr);

        let node_id = self.node_id.clone();
        let cookie = self.cookie.clone();
        let peers = Arc::clone(&self.peers);
        let streams = Arc::clone(&self.streams);

        std::thread::Builder::new()
            .name("dist-listener".into())
            .spawn(move || {
                for incoming in listener.incoming() {
                    match incoming {
                        Ok(mut stream) => {
                            match wire::handshake_accept(&mut stream, &node_id, &cookie) {
                                Ok(Some(remote_id)) => {
                                    eprintln!("[dist] Accepted connection from {}", remote_id);
                                    let peer = PeerState::new(remote_id.clone());
                                    peers.lock().unwrap().insert(remote_id.clone(), peer);
                                    let stream_clone = stream.try_clone().unwrap();
                                    streams.lock().unwrap().insert(remote_id.clone(), stream_clone);

                                    // Spawn reader thread for this peer
                                    let peers2 = Arc::clone(&peers);
                                    let streams2 = Arc::clone(&streams);
                                    let remote_id2 = remote_id.clone();
                                    std::thread::Builder::new()
                                        .name(format!("dist-peer-{}", remote_id))
                                        .spawn(move || {
                                            handle_peer_frames(&mut stream, &remote_id2, &peers2, &streams2);
                                        })
                                        .ok();
                                }
                                Ok(None) => {
                                    eprintln!("[dist] Handshake rejected from incoming connection");
                                }
                                Err(e) => {
                                    eprintln!("[dist] Handshake error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[dist] Accept error: {}", e);
                        }
                    }
                }
            })?;

        Ok(())
    }

    /// Connect to a remote peer at the given address. Performs the
    /// handshake and, on success, starts a reader thread and heartbeat
    /// monitor for the connection.
    #[allow(dead_code)]
    pub fn connect_to(&self, addr: &str) -> std::io::Result<Option<String>> {
        let mut stream = TcpStream::connect(addr)?;
        match wire::handshake_connect(&mut stream, &self.node_id, &self.cookie)? {
            Some(remote_id) => {
                eprintln!("[dist] Connected to {}", remote_id);
                let peer = PeerState::new(remote_id.clone());
                self.peers.lock().unwrap().insert(remote_id.clone(), peer);
                let stream_clone = stream.try_clone()?;
                self.streams.lock().unwrap().insert(remote_id.clone(), stream_clone);

                // Start reader thread
                let peers = Arc::clone(&self.peers);
                let streams = Arc::clone(&self.streams);
                let remote_id2 = remote_id.clone();
                std::thread::Builder::new()
                    .name(format!("dist-peer-{}", remote_id))
                    .spawn(move || {
                        handle_peer_frames(&mut stream, &remote_id2, &peers, &streams);
                    })?;

                // Start heartbeat sender
                let peers_hb = Arc::clone(&self.peers);
                let streams_hb = Arc::clone(&self.streams);
                let remote_id_hb = remote_id.clone();
                std::thread::Builder::new()
                    .name(format!("dist-hb-{}", remote_id))
                    .spawn(move || {
                        heartbeat_loop(&remote_id_hb, &peers_hb, &streams_hb);
                    })?;

                Ok(Some(remote_id))
            }
            None => {
                eprintln!("[dist] Handshake rejected by {}", addr);
                Ok(None)
            }
        }
    }

    /// Send a message to a process on a remote node.
    #[allow(dead_code)]
    pub fn send_remote(&self, peer_id: &str, target_pid: u64, message: &[u8]) -> std::io::Result<()> {
        let frame = WireFrame::send_msg(target_pid, message);
        let mut streams = self.streams.lock().unwrap();
        if let Some(stream) = streams.get_mut(peer_id) {
            frame.write_to(stream)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                format!("No connection to peer {}", peer_id),
            ));
        }
        Ok(())
    }

    /// Request a remote node to spawn a process with the given entry function.
    #[allow(dead_code)]
    pub fn spawn_remote(&self, peer_id: &str, func_name: &str) -> std::io::Result<()> {
        let frame = WireFrame::spawn_msg(func_name);
        let mut streams = self.streams.lock().unwrap();
        if let Some(stream) = streams.get_mut(peer_id) {
            frame.write_to(stream)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                format!("No connection to peer {}", peer_id),
            ));
        }
        Ok(())
    }

    /// Get the health status of a peer connection.
    #[allow(dead_code)]
    pub fn peer_health(&self, peer_id: &str) -> Option<ConnectionHealth> {
        self.peers.lock().unwrap().get(peer_id).map(|p| p.health)
    }

    /// List connected peer IDs.
    #[allow(dead_code)]
    pub fn connected_peers(&self) -> Vec<String> {
        self.peers.lock().unwrap().keys().cloned().collect()
    }
}

// =========================================================================
// Background workers
// =========================================================================

/// Read frames from a peer connection and dispatch them.
fn handle_peer_frames(
    stream: &mut TcpStream,
    remote_id: &str,
    peers: &Arc<Mutex<HashMap<String, PeerState>>>,
    streams: &Arc<Mutex<HashMap<String, TcpStream>>>,
) {
    loop {
        match WireFrame::read_from(stream) {
            Ok(Some(frame)) => {
                match frame.msg_type {
                    MSG_SEND => {
                        if let Some((pid, msg)) = frame.parse_send() {
                            eprintln!("[dist] Received SEND for pid {} from {} ({} bytes)",
                                pid, remote_id, msg.len());
                            // TODO: route to local scheduler
                        }
                    }
                    MSG_SPAWN => {
                        if let Some(func_name) = frame.parse_spawn() {
                            eprintln!("[dist] Received SPAWN request '{}' from {}",
                                func_name, remote_id);
                            // TODO: route to local scheduler
                        }
                    }
                    MSG_EXIT => {
                        if let Some(pid) = frame.parse_exit() {
                            eprintln!("[dist] Received EXIT for pid {} from {}", pid, remote_id);
                        }
                    }
                    MSG_PING => {
                        if let Some(ts) = frame.parse_heartbeat_ts() {
                            // Respond with PONG echoing the timestamp
                            let pong = WireFrame::pong(ts);
                            if let Some(s) = streams.lock().unwrap().get_mut(remote_id) {
                                let _ = pong.write_to(s);
                            }
                        }
                    }
                    MSG_PONG => {
                        if let Some(_ts) = frame.parse_heartbeat_ts() {
                            if let Some(peer) = peers.lock().unwrap().get_mut(remote_id) {
                                peer.record_pong_received();
                            }
                        }
                    }
                    other => {
                        eprintln!("[dist] Unknown message type 0x{:02x} from {}",
                            other, remote_id);
                    }
                }
            }
            Ok(None) => {
                eprintln!("[dist] Peer {} disconnected", remote_id);
                break;
            }
            Err(e) => {
                eprintln!("[dist] Read error from {}: {}", remote_id, e);
                break;
            }
        }
    }

    // Cleanup
    peers.lock().unwrap().remove(remote_id);
    streams.lock().unwrap().remove(remote_id);
    eprintln!("[dist] Removed peer {}", remote_id);
}

/// Periodically send PING frames to a peer and track missed responses.
fn heartbeat_loop(
    remote_id: &str,
    peers: &Arc<Mutex<HashMap<String, PeerState>>>,
    streams: &Arc<Mutex<HashMap<String, TcpStream>>>,
) {
    loop {
        std::thread::sleep(HEARTBEAT_INTERVAL);

        // Check if peer still exists
        let still_connected = peers.lock().unwrap().contains_key(remote_id);
        if !still_connected {
            break;
        }

        // Send PING
        let ping = WireFrame::ping();
        let send_ok = {
            if let Some(s) = streams.lock().unwrap().get_mut(remote_id) {
                ping.write_to(s).is_ok()
            } else {
                false
            }
        };

        if send_ok {
            if let Some(peer) = peers.lock().unwrap().get_mut(remote_id) {
                peer.record_ping_sent();
                if peer.health == ConnectionHealth::Unhealthy {
                    eprintln!(
                        "[dist] Dropping unhealthy connection to {}",
                        remote_id
                    );
                    break;
                }
            }
        } else {
            eprintln!("[dist] Failed to send PING to {}, dropping", remote_id);
            break;
        }
    }

    // Remove unhealthy peer
    peers.lock().unwrap().remove(remote_id);
    streams.lock().unwrap().remove(remote_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_node_id_is_deterministic() {
        let id1 = generate_node_id("localhost", 9000, Some("mynode"));
        let id2 = generate_node_id("localhost", 9000, Some("mynode"));
        assert_eq!(id1, id2);
        assert_eq!(id1, "mynode@localhost:9000");
    }

    #[test]
    fn durable_node_id_default_name() {
        let id = generate_node_id("10.0.0.1", 4370, None);
        assert_eq!(id, "japl@10.0.0.1:4370");
    }

    #[test]
    fn peer_health_tracking() {
        let mut peer = PeerState::new("test-node".to_string());
        assert_eq!(peer.health, ConnectionHealth::Healthy);

        // Miss heartbeats up to threshold
        for _ in 0..MAX_MISSED_HEARTBEATS {
            peer.record_ping_sent();
        }
        assert_eq!(peer.health, ConnectionHealth::Unhealthy);

        // Recovery
        peer.record_pong_received();
        assert_eq!(peer.health, ConnectionHealth::Healthy);
        assert_eq!(peer.missed_heartbeats, 0);
    }

    #[test]
    fn distribution_node_creation() {
        let node = DistributionNode::new("localhost", 9001, Some("test"), Some("cookie123"));
        assert_eq!(node.node_id(), "test@localhost:9001");
        assert!(node.connected_peers().is_empty());
    }
}

// =========================================================================
// JAPL Wire Protocol
// =========================================================================
//
// Wire Frame Format (all values little-endian):
// ┌──────────┬──────────┬──────────────┬───────────────┐
// │ version  │ msg_type │ payload_len  │ payload       │
// │ (1 byte) │ (1 byte) │ (4 bytes LE) │ (N bytes)     │
// └──────────┴──────────┴──────────────┴───────────────┘
//
// - version:     Protocol version byte (currently 1). On receive, if the
//                version does not match WIRE_PROTOCOL_VERSION the frame is
//                still processed but a warning is logged. This allows nodes
//                running different releases to interoperate while flagging
//                potential incompatibilities.
//
// - msg_type:    One of the MSG_* constants below.
//
// - payload_len: u32 little-endian length of the payload that follows.
//
// - payload:     Variable-length, interpretation depends on msg_type.
//
// =========================================================================
// Message Types and Payloads
// =========================================================================
//
// MSG_SEND  (0x01) — Deliver a message to a remote process.
//   payload: [target_pid: u64 LE] [message_bytes: remaining]
//
// MSG_SPAWN (0x02) — Request a remote node to spawn a process.
//   payload: [func_name_len: u32 LE] [func_name: UTF-8 bytes]
//
// MSG_EXIT  (0x03) — Notify that a process has exited.
//   payload: [pid: u64 LE]
//
// MSG_PING  (0x04) — Heartbeat request (keepalive).
//   payload: [timestamp_ms: u64 LE]
//
// MSG_PONG  (0x05) — Heartbeat response.
//   payload: [timestamp_ms: u64 LE]  (echo of the PING timestamp)
//
// =========================================================================
// Handshake Sequence
// =========================================================================
//
// When a node connects to another node, the following exchange occurs
// before any MSG_* frames are sent:
//
//   1. Connector sends: HANDSHAKE_MAGIC (4 bytes "JAPL")
//                       + WIRE_PROTOCOL_VERSION (1 byte)
//                       + node_id length (u32 LE)
//                       + node_id (UTF-8 bytes)
//                       + cookie length (u32 LE)
//                       + cookie (UTF-8 bytes)
//
//   2. Acceptor validates magic, version, and cookie.
//      - On success: replies with HANDSHAKE_OK (1 byte 0x01)
//                    + its own node_id length (u32 LE) + node_id.
//      - On failure: replies with HANDSHAKE_FAIL (1 byte 0x00) and closes.
//
//   3. After a successful handshake both sides may exchange MSG_* frames.
//      The acceptor starts the PING/PONG heartbeat cycle.
//
// =========================================================================

use std::io::{self, Read, Write};

/// Current wire protocol version. Incremented when breaking changes are
/// introduced to the frame format or handshake sequence.
pub const WIRE_PROTOCOL_VERSION: u8 = 1;

// ---- Message type tag constants -----------------------------------------

/// Deliver a message to a remote process mailbox.
pub const MSG_SEND: u8 = 0x01;
/// Request the remote node to spawn a named process.
pub const MSG_SPAWN: u8 = 0x02;
/// Notify that a process has exited.
pub const MSG_EXIT: u8 = 0x03;
/// Heartbeat request (keepalive).
pub const MSG_PING: u8 = 0x04;
/// Heartbeat response.
pub const MSG_PONG: u8 = 0x05;

// ---- Handshake constants ------------------------------------------------

/// Magic bytes sent at the start of a handshake.
pub const HANDSHAKE_MAGIC: &[u8; 4] = b"JAPL";
/// Handshake accepted.
pub const HANDSHAKE_OK: u8 = 0x01;
/// Handshake rejected.
pub const HANDSHAKE_FAIL: u8 = 0x00;

// ---- Wire frame header size ---------------------------------------------

/// Header is: version (1) + msg_type (1) + payload_len (4) = 6 bytes.
pub const WIRE_HEADER_SIZE: usize = 6;

// =========================================================================
// WireFrame — a single protocol message on the wire
// =========================================================================

/// A decoded wire frame.
#[derive(Debug, Clone)]
pub struct WireFrame {
    pub version: u8,
    pub msg_type: u8,
    pub payload: Vec<u8>,
}

impl WireFrame {
    /// Create a new frame with the current protocol version.
    pub fn new(msg_type: u8, payload: Vec<u8>) -> Self {
        Self {
            version: WIRE_PROTOCOL_VERSION,
            msg_type,
            payload,
        }
    }

    /// Encode this frame into bytes suitable for writing to a stream.
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() as u32;
        let mut buf = Vec::with_capacity(WIRE_HEADER_SIZE + self.payload.len());
        buf.push(self.version);
        buf.push(self.msg_type);
        buf.extend_from_slice(&payload_len.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Write this frame to a stream.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.encode())?;
        w.flush()
    }

    /// Read a single frame from a stream. Returns `None` on clean EOF.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Option<Self>> {
        let mut header = [0u8; WIRE_HEADER_SIZE];
        match r.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let version = header[0];
        let msg_type = header[1];
        let payload_len = u32::from_le_bytes([header[2], header[3], header[4], header[5]]) as usize;

        // Check version compatibility — log but do not reject.
        if version != WIRE_PROTOCOL_VERSION {
            eprintln!(
                "[wire] WARNING: received frame with protocol version {} (expected {})",
                version, WIRE_PROTOCOL_VERSION
            );
        }

        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            r.read_exact(&mut payload)?;
        }

        Ok(Some(Self {
            version,
            msg_type,
            payload,
        }))
    }

    // ---- Convenience constructors for each message type -----------------

    /// Build a MSG_SEND frame.
    pub fn send_msg(target_pid: u64, message_bytes: &[u8]) -> Self {
        let mut payload = Vec::with_capacity(8 + message_bytes.len());
        payload.extend_from_slice(&target_pid.to_le_bytes());
        payload.extend_from_slice(message_bytes);
        Self::new(MSG_SEND, payload)
    }

    /// Build a MSG_SPAWN frame.
    pub fn spawn_msg(func_name: &str) -> Self {
        let name_bytes = func_name.as_bytes();
        let mut payload = Vec::with_capacity(4 + name_bytes.len());
        payload.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(name_bytes);
        Self::new(MSG_SPAWN, payload)
    }

    /// Build a MSG_EXIT frame.
    pub fn exit_msg(pid: u64) -> Self {
        Self::new(MSG_EXIT, pid.to_le_bytes().to_vec())
    }

    /// Build a MSG_PING frame with the current timestamp.
    pub fn ping() -> Self {
        let ts = current_timestamp_ms();
        Self::new(MSG_PING, ts.to_le_bytes().to_vec())
    }

    /// Build a MSG_PONG frame echoing the given timestamp.
    pub fn pong(timestamp_ms: u64) -> Self {
        Self::new(MSG_PONG, timestamp_ms.to_le_bytes().to_vec())
    }

    // ---- Payload parsers ------------------------------------------------

    /// Parse a MSG_SEND payload into (target_pid, message_bytes).
    pub fn parse_send(&self) -> Option<(u64, Vec<u8>)> {
        if self.msg_type != MSG_SEND || self.payload.len() < 8 {
            return None;
        }
        let pid = u64::from_le_bytes(self.payload[..8].try_into().ok()?);
        let msg = self.payload[8..].to_vec();
        Some((pid, msg))
    }

    /// Parse a MSG_SPAWN payload into the function name.
    pub fn parse_spawn(&self) -> Option<String> {
        if self.msg_type != MSG_SPAWN || self.payload.len() < 4 {
            return None;
        }
        let len = u32::from_le_bytes(self.payload[..4].try_into().ok()?) as usize;
        if self.payload.len() < 4 + len {
            return None;
        }
        String::from_utf8(self.payload[4..4 + len].to_vec()).ok()
    }

    /// Parse a MSG_EXIT payload into the pid.
    pub fn parse_exit(&self) -> Option<u64> {
        if self.msg_type != MSG_EXIT || self.payload.len() < 8 {
            return None;
        }
        Some(u64::from_le_bytes(self.payload[..8].try_into().ok()?))
    }

    /// Parse a MSG_PING or MSG_PONG payload into the timestamp.
    pub fn parse_heartbeat_ts(&self) -> Option<u64> {
        if self.payload.len() < 8 {
            return None;
        }
        Some(u64::from_le_bytes(self.payload[..8].try_into().ok()?))
    }
}

// =========================================================================
// Handshake helpers
// =========================================================================

/// Perform the connector side of the handshake: send magic + version +
/// node_id + cookie, then read the response.
pub fn handshake_connect<S: Read + Write>(
    stream: &mut S,
    node_id: &str,
    cookie: &str,
) -> io::Result<Option<String>> {
    // Send: MAGIC + VERSION + node_id + cookie
    stream.write_all(HANDSHAKE_MAGIC)?;
    stream.write_all(&[WIRE_PROTOCOL_VERSION])?;
    write_length_prefixed(stream, node_id.as_bytes())?;
    write_length_prefixed(stream, cookie.as_bytes())?;
    stream.flush()?;

    // Read response
    let mut status = [0u8; 1];
    stream.read_exact(&mut status)?;
    if status[0] == HANDSHAKE_OK {
        let remote_id = read_length_prefixed(stream)?;
        let remote_id = String::from_utf8(remote_id)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(remote_id))
    } else {
        Ok(None)
    }
}

/// Perform the acceptor side of the handshake: read magic + version +
/// node_id + cookie, validate, respond.
pub fn handshake_accept<S: Read + Write>(
    stream: &mut S,
    own_node_id: &str,
    expected_cookie: &str,
) -> io::Result<Option<String>> {
    // Read magic
    let mut magic = [0u8; 4];
    stream.read_exact(&mut magic)?;
    if &magic != HANDSHAKE_MAGIC {
        stream.write_all(&[HANDSHAKE_FAIL])?;
        stream.flush()?;
        return Ok(None);
    }

    // Read version
    let mut ver = [0u8; 1];
    stream.read_exact(&mut ver)?;
    if ver[0] != WIRE_PROTOCOL_VERSION {
        eprintln!(
            "[wire] WARNING: handshake version mismatch: got {} expected {}",
            ver[0], WIRE_PROTOCOL_VERSION
        );
        // Continue anyway — version mismatches are warnings, not errors.
    }

    // Read remote node_id
    let remote_id_bytes = read_length_prefixed(stream)?;
    let remote_id = String::from_utf8(remote_id_bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Read cookie
    let cookie_bytes = read_length_prefixed(stream)?;
    let cookie = String::from_utf8(cookie_bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if cookie != expected_cookie {
        eprintln!("[wire] Handshake rejected: bad cookie from {}", remote_id);
        stream.write_all(&[HANDSHAKE_FAIL])?;
        stream.flush()?;
        return Ok(None);
    }

    // Accept — send OK + our node_id
    stream.write_all(&[HANDSHAKE_OK])?;
    write_length_prefixed(stream, own_node_id.as_bytes())?;
    stream.flush()?;

    Ok(Some(remote_id))
}

// =========================================================================
// Internal helpers
// =========================================================================

fn write_length_prefixed<W: Write>(w: &mut W, data: &[u8]) -> io::Result<()> {
    w.write_all(&(data.len() as u32).to_le_bytes())?;
    w.write_all(data)
}

fn read_length_prefixed<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    if len > 0 {
        r.read_exact(&mut buf)?;
    }
    Ok(buf)
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Return a human-readable label for a message type tag.
pub fn msg_type_name(tag: u8) -> &'static str {
    match tag {
        MSG_SEND => "SEND",
        MSG_SPAWN => "SPAWN",
        MSG_EXIT => "EXIT",
        MSG_PING => "PING",
        MSG_PONG => "PONG",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip_frame() {
        let frame = WireFrame::send_msg(42, b"hello");
        let encoded = frame.encode();
        let mut cursor = Cursor::new(encoded);
        let decoded = WireFrame::read_from(&mut cursor).unwrap().unwrap();
        assert_eq!(decoded.version, WIRE_PROTOCOL_VERSION);
        assert_eq!(decoded.msg_type, MSG_SEND);
        let (pid, msg) = decoded.parse_send().unwrap();
        assert_eq!(pid, 42);
        assert_eq!(msg, b"hello");
    }

    #[test]
    fn roundtrip_ping_pong() {
        let ping = WireFrame::ping();
        let encoded = ping.encode();
        let mut cursor = Cursor::new(encoded);
        let decoded = WireFrame::read_from(&mut cursor).unwrap().unwrap();
        assert_eq!(decoded.msg_type, MSG_PING);
        let ts = decoded.parse_heartbeat_ts().unwrap();
        assert!(ts > 0);
    }

    #[test]
    fn handshake_roundtrip() {
        // Simulate handshake in memory using paired cursors.
        let mut connector_buf: Vec<u8> = Vec::new();

        // Connector writes handshake
        connector_buf.extend_from_slice(HANDSHAKE_MAGIC);
        connector_buf.push(WIRE_PROTOCOL_VERSION);
        let node_id = b"node-abc";
        connector_buf.extend_from_slice(&(node_id.len() as u32).to_le_bytes());
        connector_buf.extend_from_slice(node_id);
        let cookie = b"secret";
        connector_buf.extend_from_slice(&(cookie.len() as u32).to_le_bytes());
        connector_buf.extend_from_slice(cookie);

        // Acceptor reads from that buffer
        let mut read_cursor = Cursor::new(connector_buf);
        let mut response_buf: Vec<u8> = Vec::new();

        // Manually do acceptor logic
        let mut magic = [0u8; 4];
        read_cursor.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, HANDSHAKE_MAGIC);

        let mut ver = [0u8; 1];
        read_cursor.read_exact(&mut ver).unwrap();
        assert_eq!(ver[0], WIRE_PROTOCOL_VERSION);

        let remote_id = read_length_prefixed(&mut read_cursor).unwrap();
        assert_eq!(remote_id, b"node-abc");

        let cookie_read = read_length_prefixed(&mut read_cursor).unwrap();
        assert_eq!(cookie_read, b"secret");

        // Acceptor writes OK + own id
        response_buf.push(HANDSHAKE_OK);
        write_length_prefixed(&mut response_buf, b"node-xyz").unwrap();

        // Connector reads response
        let mut resp_cursor = Cursor::new(response_buf);
        let mut status = [0u8; 1];
        resp_cursor.read_exact(&mut status).unwrap();
        assert_eq!(status[0], HANDSHAKE_OK);
        let acceptor_id = read_length_prefixed(&mut resp_cursor).unwrap();
        assert_eq!(acceptor_id, b"node-xyz");
    }
}

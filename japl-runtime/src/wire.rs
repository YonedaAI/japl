/// Wire protocol for TCP distribution.
///
/// Frame format: [4 bytes big-endian length][1 byte msg_type][payload]
///
/// Message types:
///   0x01 = Handshake
///   0x02 = HandshakeOk
///   0x03 = Send
///   0x04 = SpawnRequest
///   0x05 = SpawnResponse
///   0x06 = Ping
///   0x07 = Pong

use std::io::{self, Read, Write};
use std::net::TcpStream;

#[derive(Debug, Clone)]
pub enum WireMessage {
    Handshake { node_name: String, cookie: String },
    HandshakeOk { node_name: String },
    Send { to_pid: u64, from_pid: u64, msg: i64 },
    SpawnRequest { request_id: u64 },
    SpawnResponse { request_id: u64, pid: u64 },
    Ping,
    Pong,
}

const MSG_HANDSHAKE: u8 = 0x01;
const MSG_HANDSHAKE_OK: u8 = 0x02;
const MSG_SEND: u8 = 0x03;
const MSG_SPAWN_REQUEST: u8 = 0x04;
const MSG_SPAWN_RESPONSE: u8 = 0x05;
const MSG_PING: u8 = 0x06;
const MSG_PONG: u8 = 0x07;

pub fn encode(msg: &WireMessage) -> Vec<u8> {
    let mut payload = Vec::new();

    let msg_type = match msg {
        WireMessage::Handshake { node_name, cookie } => {
            let name_bytes = node_name.as_bytes();
            payload.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
            payload.extend_from_slice(name_bytes);
            let cookie_bytes = cookie.as_bytes();
            payload.extend_from_slice(&(cookie_bytes.len() as u16).to_be_bytes());
            payload.extend_from_slice(cookie_bytes);
            MSG_HANDSHAKE
        }
        WireMessage::HandshakeOk { node_name } => {
            let name_bytes = node_name.as_bytes();
            payload.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
            payload.extend_from_slice(name_bytes);
            MSG_HANDSHAKE_OK
        }
        WireMessage::Send { to_pid, from_pid, msg } => {
            payload.extend_from_slice(&to_pid.to_be_bytes());
            payload.extend_from_slice(&from_pid.to_be_bytes());
            payload.extend_from_slice(&msg.to_be_bytes());
            MSG_SEND
        }
        WireMessage::SpawnRequest { request_id } => {
            payload.extend_from_slice(&request_id.to_be_bytes());
            MSG_SPAWN_REQUEST
        }
        WireMessage::SpawnResponse { request_id, pid } => {
            payload.extend_from_slice(&request_id.to_be_bytes());
            payload.extend_from_slice(&pid.to_be_bytes());
            MSG_SPAWN_RESPONSE
        }
        WireMessage::Ping => MSG_PING,
        WireMessage::Pong => MSG_PONG,
    };

    // Frame: [4 bytes length of (msg_type + payload)][1 byte msg_type][payload]
    let frame_len = 1 + payload.len();
    let mut frame = Vec::with_capacity(4 + frame_len);
    frame.extend_from_slice(&(frame_len as u32).to_be_bytes());
    frame.push(msg_type);
    frame.extend_from_slice(&payload);
    frame
}

pub fn decode(bytes: &[u8]) -> anyhow::Result<WireMessage> {
    if bytes.is_empty() {
        anyhow::bail!("empty message");
    }

    let msg_type = bytes[0];
    let payload = &bytes[1..];

    match msg_type {
        MSG_HANDSHAKE => {
            if payload.len() < 2 {
                anyhow::bail!("handshake too short");
            }
            let name_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;
            if payload.len() < 2 + name_len + 2 {
                anyhow::bail!("handshake truncated");
            }
            let node_name = String::from_utf8(payload[2..2 + name_len].to_vec())?;
            let cookie_offset = 2 + name_len;
            let cookie_len =
                u16::from_be_bytes([payload[cookie_offset], payload[cookie_offset + 1]]) as usize;
            if payload.len() < cookie_offset + 2 + cookie_len {
                anyhow::bail!("handshake cookie truncated");
            }
            let cookie = String::from_utf8(
                payload[cookie_offset + 2..cookie_offset + 2 + cookie_len].to_vec(),
            )?;
            Ok(WireMessage::Handshake { node_name, cookie })
        }
        MSG_HANDSHAKE_OK => {
            if payload.len() < 2 {
                anyhow::bail!("handshake_ok too short");
            }
            let name_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;
            if payload.len() < 2 + name_len {
                anyhow::bail!("handshake_ok truncated");
            }
            let node_name = String::from_utf8(payload[2..2 + name_len].to_vec())?;
            Ok(WireMessage::HandshakeOk { node_name })
        }
        MSG_SEND => {
            if payload.len() < 24 {
                anyhow::bail!("send too short");
            }
            let to_pid = u64::from_be_bytes(payload[0..8].try_into()?);
            let from_pid = u64::from_be_bytes(payload[8..16].try_into()?);
            let msg = i64::from_be_bytes(payload[16..24].try_into()?);
            Ok(WireMessage::Send {
                to_pid,
                from_pid,
                msg,
            })
        }
        MSG_SPAWN_REQUEST => {
            if payload.len() < 8 {
                anyhow::bail!("spawn_request too short");
            }
            let request_id = u64::from_be_bytes(payload[0..8].try_into()?);
            Ok(WireMessage::SpawnRequest { request_id })
        }
        MSG_SPAWN_RESPONSE => {
            if payload.len() < 16 {
                anyhow::bail!("spawn_response too short");
            }
            let request_id = u64::from_be_bytes(payload[0..8].try_into()?);
            let pid = u64::from_be_bytes(payload[8..16].try_into()?);
            Ok(WireMessage::SpawnResponse { request_id, pid })
        }
        MSG_PING => Ok(WireMessage::Ping),
        MSG_PONG => Ok(WireMessage::Pong),
        _ => anyhow::bail!("unknown message type: {}", msg_type),
    }
}

/// Read exactly one framed message from a TcpStream.
pub fn read_frame(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len == 0 || len > 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad frame length"));
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

/// Write one framed message to a TcpStream (encode handles framing).
pub fn write_msg(stream: &mut TcpStream, msg: &WireMessage) -> io::Result<()> {
    let bytes = encode(msg);
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

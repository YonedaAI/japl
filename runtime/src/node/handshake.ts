// Handshake authentication protocol for JAPL node connections.
//
// Flow:
// 1. Connector sends: HANDSHAKE { nodeName, cookie, version: 1 }
// 2. Listener verifies cookie matches
// 3. If match: send HANDSHAKE_ACK { nodeName }
// 4. If no match: send HANDSHAKE_NACK { reason }, close

import {
  MsgType,
  type WireMessage,
  type HandshakePayload,
  encodeFrame,
  encodeHandshakePayload,
  decodeHandshakePayload,
} from "../wire/index.js";

const encoder = new TextEncoder();

const HANDSHAKE_VERSION = 1;

/** Create a HANDSHAKE wire message. */
export function createHandshakeMessage(nodeName: string, cookie: string): Uint8Array {
  const payload = encodeHandshakePayload({ nodeName, cookie, version: HANDSHAKE_VERSION });
  const msg: WireMessage = {
    type: MsgType.HANDSHAKE,
    fromNode: nodeName,
    toNode: "",
    payload,
  };
  return encodeFrame(msg);
}

/** Create a HANDSHAKE_ACK wire message. */
export function createHandshakeAck(nodeName: string): Uint8Array {
  // Payload is just the responder's node name (length-prefixed).
  const nameBytes = encoder.encode(nodeName);
  const payload = new Uint8Array(2 + nameBytes.length);
  new DataView(payload.buffer).setUint16(0, nameBytes.length);
  payload.set(nameBytes, 2);
  const msg: WireMessage = {
    type: MsgType.HANDSHAKE_ACK,
    fromNode: nodeName,
    toNode: "",
    payload,
  };
  return encodeFrame(msg);
}

/** Create a HANDSHAKE_NACK wire message. */
export function createHandshakeNack(nodeName: string, reason: string): Uint8Array {
  const reasonBytes = encoder.encode(reason);
  const payload = new Uint8Array(2 + reasonBytes.length);
  new DataView(payload.buffer).setUint16(0, reasonBytes.length);
  payload.set(reasonBytes, 2);
  const msg: WireMessage = {
    type: MsgType.HANDSHAKE_NACK,
    fromNode: nodeName,
    toNode: "",
    payload,
  };
  return encodeFrame(msg);
}

/** Verify an incoming handshake payload against the expected cookie. */
export function verifyHandshake(
  payload: Uint8Array,
  expectedCookie: string,
): { valid: boolean; nodeName: string; reason?: string } {
  try {
    const hs: HandshakePayload = decodeHandshakePayload(payload);
    if (hs.cookie !== expectedCookie) {
      return { valid: false, nodeName: hs.nodeName, reason: "bad cookie" };
    }
    if (hs.version !== HANDSHAKE_VERSION) {
      return { valid: false, nodeName: hs.nodeName, reason: `unsupported version: ${hs.version}` };
    }
    return { valid: true, nodeName: hs.nodeName };
  } catch {
    return { valid: false, nodeName: "", reason: "malformed handshake payload" };
  }
}

import type {
  WireMessage,
  SendPayload,
  SpawnRequestPayload,
  SpawnResponsePayload,
  ExitPayload,
  HandshakePayload,
  RegisterPayload,
  LookupPayload,
  LookupResponsePayload,
  LinkPayload,
  MonitorPayload,
} from "./protocol.js";
import { MsgType } from "./protocol.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

// ---------------------------------------------------------------------------
// Frame encode / decode
// ---------------------------------------------------------------------------

export function encodeFrame(msg: WireMessage): Uint8Array {
  const fromBytes = encoder.encode(msg.fromNode);
  const toBytes = encoder.encode(msg.toNode);
  const totalLen = 4 + 1 + 2 + fromBytes.length + 2 + toBytes.length + msg.payload.length;

  const buf = new Uint8Array(totalLen);
  const view = new DataView(buf.buffer);
  let offset = 0;

  view.setUint32(offset, totalLen); offset += 4;
  buf[offset] = msg.type; offset += 1;
  view.setUint16(offset, fromBytes.length); offset += 2;
  buf.set(fromBytes, offset); offset += fromBytes.length;
  view.setUint16(offset, toBytes.length); offset += 2;
  buf.set(toBytes, offset); offset += toBytes.length;
  buf.set(msg.payload, offset);

  return buf;
}

export function decodeFrame(buf: Uint8Array): WireMessage {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;

  const totalLen = view.getUint32(offset); offset += 4;
  const type = buf[offset] as MsgType; offset += 1;

  const fromLen = view.getUint16(offset); offset += 2;
  const fromNode = decoder.decode(buf.subarray(offset, offset + fromLen)); offset += fromLen;

  const toLen = view.getUint16(offset); offset += 2;
  const toNode = decoder.decode(buf.subarray(offset, offset + toLen)); offset += toLen;

  const payload = buf.slice(offset, totalLen);

  return { type, fromNode, toNode, payload };
}

// ---------------------------------------------------------------------------
// FrameReader — reassembles frames from a TCP byte stream
// ---------------------------------------------------------------------------

export class FrameReader {
  private buffer: Uint8Array = new Uint8Array(0);

  feed(data: Uint8Array): WireMessage[] {
    const combined = new Uint8Array(this.buffer.length + data.length);
    combined.set(this.buffer);
    combined.set(data, this.buffer.length);
    this.buffer = combined;

    const messages: WireMessage[] = [];

    while (this.buffer.length >= 4) {
      const view = new DataView(this.buffer.buffer, this.buffer.byteOffset);
      const frameLen = view.getUint32(0);

      if (frameLen < 4) break; // malformed
      if (this.buffer.length < frameLen) break; // incomplete

      const frame = this.buffer.slice(0, frameLen);
      messages.push(decodeFrame(frame));
      this.buffer = this.buffer.subarray(frameLen);
    }

    return messages;
  }
}

// ---------------------------------------------------------------------------
// Helpers — length-prefixed UTF-8 strings, big-endian numbers
// ---------------------------------------------------------------------------

function writeString(parts: Uint8Array[], s: string): void {
  const bytes = encoder.encode(s);
  const len = new Uint8Array(2);
  new DataView(len.buffer).setUint16(0, bytes.length);
  parts.push(len);
  parts.push(bytes);
}

function readString(view: DataView, buf: Uint8Array, offset: number): [string, number] {
  const len = view.getUint16(offset); offset += 2;
  const s = decoder.decode(buf.subarray(offset, offset + len)); offset += len;
  return [s, offset];
}

function writeUint32(parts: Uint8Array[], n: number): void {
  const b = new Uint8Array(4);
  new DataView(b.buffer).setUint32(0, n);
  parts.push(b);
}

function readUint32(view: DataView, offset: number): [number, number] {
  return [view.getUint32(offset), offset + 4];
}

function writeBytes(parts: Uint8Array[], data: Uint8Array): void {
  const len = new Uint8Array(4);
  new DataView(len.buffer).setUint32(0, data.length);
  parts.push(len);
  parts.push(data);
}

function readBytes(view: DataView, buf: Uint8Array, offset: number): [Uint8Array, number] {
  const len = view.getUint32(offset); offset += 4;
  const data = buf.slice(offset, offset + len); offset += len;
  return [data, offset];
}

function concat(parts: Uint8Array[]): Uint8Array {
  let total = 0;
  for (const p of parts) total += p.length;
  const result = new Uint8Array(total);
  let off = 0;
  for (const p of parts) { result.set(p, off); off += p.length; }
  return result;
}

// ---------------------------------------------------------------------------
// SendPayload
// ---------------------------------------------------------------------------

export function encodeSendPayload(p: SendPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.toPid);
  writeString(parts, p.fromPid);
  writeBytes(parts, p.data);
  return concat(parts);
}

export function decodeSendPayload(buf: Uint8Array): SendPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let toPid: string, fromPid: string, data: Uint8Array;
  [toPid, offset] = readString(view, buf, offset);
  [fromPid, offset] = readString(view, buf, offset);
  [data, offset] = readBytes(view, buf, offset);
  return { toPid, fromPid, data };
}

// ---------------------------------------------------------------------------
// SpawnRequestPayload
// ---------------------------------------------------------------------------

export function encodeSpawnRequestPayload(p: SpawnRequestPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.requestId);
  writeString(parts, p.module);
  writeString(parts, p.fn);
  writeBytes(parts, p.args);
  return concat(parts);
}

export function decodeSpawnRequestPayload(buf: Uint8Array): SpawnRequestPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let requestId: string, module: string, fn: string, args: Uint8Array;
  [requestId, offset] = readString(view, buf, offset);
  [module, offset] = readString(view, buf, offset);
  [fn, offset] = readString(view, buf, offset);
  [args, offset] = readBytes(view, buf, offset);
  return { requestId, module, fn, args };
}

// ---------------------------------------------------------------------------
// SpawnResponsePayload
// ---------------------------------------------------------------------------

export function encodeSpawnResponsePayload(p: SpawnResponsePayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.requestId);
  writeString(parts, p.pid);
  return concat(parts);
}

export function decodeSpawnResponsePayload(buf: Uint8Array): SpawnResponsePayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let requestId: string, pid: string;
  [requestId, offset] = readString(view, buf, offset);
  [pid, offset] = readString(view, buf, offset);
  return { requestId, pid };
}

// ---------------------------------------------------------------------------
// ExitPayload
// ---------------------------------------------------------------------------

export function encodeExitPayload(p: ExitPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.pid);
  writeString(parts, p.reason);
  return concat(parts);
}

export function decodeExitPayload(buf: Uint8Array): ExitPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let pid: string, reason: string;
  [pid, offset] = readString(view, buf, offset);
  [reason, offset] = readString(view, buf, offset);
  return { pid, reason };
}

// ---------------------------------------------------------------------------
// HandshakePayload
// ---------------------------------------------------------------------------

export function encodeHandshakePayload(p: HandshakePayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.nodeName);
  writeString(parts, p.cookie);
  writeUint32(parts, p.version);
  return concat(parts);
}

export function decodeHandshakePayload(buf: Uint8Array): HandshakePayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let nodeName: string, cookie: string, version: number;
  [nodeName, offset] = readString(view, buf, offset);
  [cookie, offset] = readString(view, buf, offset);
  [version, offset] = readUint32(view, offset);
  return { nodeName, cookie, version };
}

// ---------------------------------------------------------------------------
// RegisterPayload
// ---------------------------------------------------------------------------

export function encodeRegisterPayload(p: RegisterPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.name);
  writeString(parts, p.pid);
  return concat(parts);
}

export function decodeRegisterPayload(buf: Uint8Array): RegisterPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let name: string, pid: string;
  [name, offset] = readString(view, buf, offset);
  [pid, offset] = readString(view, buf, offset);
  return { name, pid };
}

// ---------------------------------------------------------------------------
// LookupPayload
// ---------------------------------------------------------------------------

export function encodeLookupPayload(p: LookupPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.requestId);
  writeString(parts, p.name);
  return concat(parts);
}

export function decodeLookupPayload(buf: Uint8Array): LookupPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let requestId: string, name: string;
  [requestId, offset] = readString(view, buf, offset);
  [name, offset] = readString(view, buf, offset);
  return { requestId, name };
}

// ---------------------------------------------------------------------------
// LookupResponsePayload
// ---------------------------------------------------------------------------

export function encodeLookupResponsePayload(p: LookupResponsePayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.requestId);
  // Use a flag byte: 0x00 = null, 0x01 = present
  if (p.pid === null) {
    parts.push(new Uint8Array([0x00]));
  } else {
    parts.push(new Uint8Array([0x01]));
    writeString(parts, p.pid);
  }
  return concat(parts);
}

export function decodeLookupResponsePayload(buf: Uint8Array): LookupResponsePayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let requestId: string;
  [requestId, offset] = readString(view, buf, offset);
  const flag = buf[offset]; offset += 1;
  let pid: string | null = null;
  if (flag === 0x01) {
    [pid, offset] = readString(view, buf, offset);
  }
  return { requestId, pid };
}

// ---------------------------------------------------------------------------
// LinkPayload
// ---------------------------------------------------------------------------

export function encodeLinkPayload(p: LinkPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.fromPid);
  writeString(parts, p.toPid);
  return concat(parts);
}

export function decodeLinkPayload(buf: Uint8Array): LinkPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let fromPid: string, toPid: string;
  [fromPid, offset] = readString(view, buf, offset);
  [toPid, offset] = readString(view, buf, offset);
  return { fromPid, toPid };
}

// ---------------------------------------------------------------------------
// MonitorPayload
// ---------------------------------------------------------------------------

export function encodeMonitorPayload(p: MonitorPayload): Uint8Array {
  const parts: Uint8Array[] = [];
  writeString(parts, p.monitorPid);
  writeString(parts, p.targetPid);
  return concat(parts);
}

export function decodeMonitorPayload(buf: Uint8Array): MonitorPayload {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;
  let monitorPid: string, targetPid: string;
  [monitorPid, offset] = readString(view, buf, offset);
  [targetPid, offset] = readString(view, buf, offset);
  return { monitorPid, targetPid };
}

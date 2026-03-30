// Distributed Process ID — uniquely identifies a process across the cluster.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export interface DistributedPid {
  node: string;   // node name (e.g., "alpha")
  local: string;  // local process UUID
}

export function makePid(node: string, local: string): DistributedPid {
  return { node, local };
}

export function pidToString(pid: DistributedPid): string {
  return `${pid.node}:${pid.local}`;
}

export function parsePid(s: string): DistributedPid {
  const idx = s.indexOf(":");
  if (idx < 0) {
    throw new Error(`Invalid distributed PID: ${s}`);
  }
  return { node: s.slice(0, idx), local: s.slice(idx + 1) };
}

export function isLocal(pid: DistributedPid, selfNode: string): boolean {
  return pid.node === selfNode;
}

export function serializePid(pid: DistributedPid): Uint8Array {
  const nodeBytes = encoder.encode(pid.node);
  const localBytes = encoder.encode(pid.local);
  const buf = new Uint8Array(2 + nodeBytes.length + 2 + localBytes.length);
  const view = new DataView(buf.buffer);
  let offset = 0;
  view.setUint16(offset, nodeBytes.length); offset += 2;
  buf.set(nodeBytes, offset); offset += nodeBytes.length;
  view.setUint16(offset, localBytes.length); offset += 2;
  buf.set(localBytes, offset);
  return buf;
}

export function deserializePid(buf: Uint8Array): { pid: DistributedPid; bytesRead: number } {
  const view = new DataView(buf.buffer, buf.byteOffset);
  let offset = 0;

  const nodeLen = view.getUint16(offset); offset += 2;
  const node = decoder.decode(buf.subarray(offset, offset + nodeLen)); offset += nodeLen;

  const localLen = view.getUint16(offset); offset += 2;
  const local = decoder.decode(buf.subarray(offset, offset + localLen)); offset += localLen;

  return { pid: { node, local }, bytesRead: offset };
}

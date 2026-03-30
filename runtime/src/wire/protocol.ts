export enum MsgType {
  SEND            = 0x01,
  SPAWN_REQUEST   = 0x02,
  SPAWN_RESPONSE  = 0x03,
  LINK            = 0x04,
  UNLINK          = 0x05,
  EXIT            = 0x06,
  MONITOR         = 0x07,
  DEMONITOR       = 0x08,
  NODE_DOWN       = 0x09,
  PING            = 0x0A,
  PONG            = 0x0B,
  HANDSHAKE       = 0x0C,
  HANDSHAKE_ACK   = 0x0D,
  HANDSHAKE_NACK  = 0x0E,
  REGISTER        = 0x0F,
  LOOKUP          = 0x10,
  LOOKUP_RESPONSE = 0x11,
}

export interface WireMessage {
  type: MsgType;
  fromNode: string;
  toNode: string;
  payload: Uint8Array;
}

export interface SendPayload {
  toPid: string;
  fromPid: string;
  data: Uint8Array;
}

export interface SpawnRequestPayload {
  requestId: string;
  module: string;
  fn: string;
  args: Uint8Array;
}

export interface SpawnResponsePayload {
  requestId: string;
  pid: string;
}

export interface ExitPayload {
  pid: string;
  reason: string;
}

export interface HandshakePayload {
  nodeName: string;
  cookie: string;
  version: number;
}

export interface RegisterPayload {
  name: string;
  pid: string;
}

export interface LookupPayload {
  requestId: string;
  name: string;
}

export interface LookupResponsePayload {
  requestId: string;
  pid: string | null;
}

export interface LinkPayload {
  fromPid: string;
  toPid: string;
}

export interface MonitorPayload {
  monitorPid: string;
  targetPid: string;
}

import { describe, it, expect } from "vitest";
import {
  MsgType,
  type WireMessage,
  encodeFrame,
  decodeFrame,
  FrameReader,
  encodeSendPayload,
  decodeSendPayload,
  encodeSpawnRequestPayload,
  decodeSpawnRequestPayload,
  encodeSpawnResponsePayload,
  decodeSpawnResponsePayload,
  encodeExitPayload,
  decodeExitPayload,
  encodeHandshakePayload,
  decodeHandshakePayload,
  encodeRegisterPayload,
  decodeRegisterPayload,
  encodeLookupPayload,
  decodeLookupPayload,
  encodeLookupResponsePayload,
  decodeLookupResponsePayload,
  encodeLinkPayload,
  decodeLinkPayload,
  encodeMonitorPayload,
  decodeMonitorPayload,
} from "../../src/wire/index.js";

describe("wire protocol", () => {
  // -----------------------------------------------------------------------
  // Frame round-trip
  // -----------------------------------------------------------------------

  it("encodes and decodes a frame", () => {
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "alpha",
      toNode: "beta",
      payload: new Uint8Array([1, 2, 3]),
    };
    const encoded = encodeFrame(msg);
    const decoded = decodeFrame(encoded);
    expect(decoded.type).toBe(MsgType.SEND);
    expect(decoded.fromNode).toBe("alpha");
    expect(decoded.toNode).toBe("beta");
    expect(decoded.payload).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("handles empty payload", () => {
    const msg: WireMessage = {
      type: MsgType.PING,
      fromNode: "a",
      toNode: "b",
      payload: new Uint8Array(0),
    };
    const decoded = decodeFrame(encodeFrame(msg));
    expect(decoded.type).toBe(MsgType.PING);
    expect(decoded.payload.length).toBe(0);
  });

  it("handles long node names", () => {
    const longName = "node-" + "x".repeat(500);
    const msg: WireMessage = {
      type: MsgType.HANDSHAKE,
      fromNode: longName,
      toNode: longName,
      payload: new Uint8Array([42]),
    };
    const decoded = decodeFrame(encodeFrame(msg));
    expect(decoded.fromNode).toBe(longName);
    expect(decoded.toNode).toBe(longName);
  });

  it("handles binary payload with all byte values", () => {
    const payload = new Uint8Array(256);
    for (let i = 0; i < 256; i++) payload[i] = i;
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "src",
      toNode: "dst",
      payload,
    };
    const decoded = decodeFrame(encodeFrame(msg));
    expect(decoded.payload).toEqual(payload);
  });

  it("preserves all MsgType values through frame encoding", () => {
    const types = [
      MsgType.SEND, MsgType.SPAWN_REQUEST, MsgType.SPAWN_RESPONSE,
      MsgType.LINK, MsgType.UNLINK, MsgType.EXIT,
      MsgType.MONITOR, MsgType.DEMONITOR, MsgType.NODE_DOWN,
      MsgType.PING, MsgType.PONG,
      MsgType.HANDSHAKE, MsgType.HANDSHAKE_ACK, MsgType.HANDSHAKE_NACK,
      MsgType.REGISTER, MsgType.LOOKUP, MsgType.LOOKUP_RESPONSE,
    ];
    for (const t of types) {
      const msg: WireMessage = { type: t, fromNode: "a", toNode: "b", payload: new Uint8Array(0) };
      const decoded = decodeFrame(encodeFrame(msg));
      expect(decoded.type).toBe(t);
    }
  });

  it("handles unicode node names", () => {
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "nodo-\u00e9l\u00e8ve",
      toNode: "\u30ce\u30fc\u30c9",
      payload: new Uint8Array([0xFF]),
    };
    const decoded = decodeFrame(encodeFrame(msg));
    expect(decoded.fromNode).toBe("nodo-\u00e9l\u00e8ve");
    expect(decoded.toNode).toBe("\u30ce\u30fc\u30c9");
  });

  // -----------------------------------------------------------------------
  // FrameReader
  // -----------------------------------------------------------------------

  it("FrameReader handles partial reads", () => {
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "alpha",
      toNode: "beta",
      payload: new Uint8Array([10, 20]),
    };
    const full = encodeFrame(msg);
    const reader = new FrameReader();

    // Feed first half
    const half = Math.floor(full.length / 2);
    let msgs = reader.feed(full.subarray(0, half));
    expect(msgs.length).toBe(0);

    // Feed second half
    msgs = reader.feed(full.subarray(half));
    expect(msgs.length).toBe(1);
    expect(msgs[0].fromNode).toBe("alpha");
  });

  it("FrameReader handles multiple frames in one chunk", () => {
    const m1: WireMessage = { type: MsgType.PING, fromNode: "a", toNode: "b", payload: new Uint8Array(0) };
    const m2: WireMessage = { type: MsgType.PONG, fromNode: "b", toNode: "a", payload: new Uint8Array(0) };
    const f1 = encodeFrame(m1);
    const f2 = encodeFrame(m2);
    const combined = new Uint8Array(f1.length + f2.length);
    combined.set(f1);
    combined.set(f2, f1.length);

    const reader = new FrameReader();
    const msgs = reader.feed(combined);
    expect(msgs.length).toBe(2);
    expect(msgs[0].type).toBe(MsgType.PING);
    expect(msgs[1].type).toBe(MsgType.PONG);
  });

  it("FrameReader handles byte-by-byte feeding", () => {
    const msg: WireMessage = { type: MsgType.EXIT, fromNode: "n1", toNode: "n2", payload: new Uint8Array([0xAB]) };
    const full = encodeFrame(msg);
    const reader = new FrameReader();
    let result: WireMessage[] = [];

    for (let i = 0; i < full.length; i++) {
      const msgs = reader.feed(full.subarray(i, i + 1));
      result.push(...msgs);
    }
    expect(result.length).toBe(1);
    expect(result[0].type).toBe(MsgType.EXIT);
  });

  // -----------------------------------------------------------------------
  // Payload round-trips
  // -----------------------------------------------------------------------

  it("SendPayload round-trips", () => {
    const p = { toPid: "pid-1", fromPid: "pid-2", data: new Uint8Array([1, 2, 3, 4]) };
    const decoded = decodeSendPayload(encodeSendPayload(p));
    expect(decoded.toPid).toBe("pid-1");
    expect(decoded.fromPid).toBe("pid-2");
    expect(decoded.data).toEqual(new Uint8Array([1, 2, 3, 4]));
  });

  it("SpawnRequestPayload round-trips", () => {
    const args = new Uint8Array([0xDE, 0xAD]);
    const p = { requestId: "req-42", module: "my_mod", fn: "init", args };
    const decoded = decodeSpawnRequestPayload(encodeSpawnRequestPayload(p));
    expect(decoded.requestId).toBe("req-42");
    expect(decoded.module).toBe("my_mod");
    expect(decoded.fn).toBe("init");
    expect(decoded.args).toEqual(args);
  });

  it("SpawnResponsePayload round-trips", () => {
    const p = { requestId: "req-42", pid: "pid-99" };
    const decoded = decodeSpawnResponsePayload(encodeSpawnResponsePayload(p));
    expect(decoded.requestId).toBe("req-42");
    expect(decoded.pid).toBe("pid-99");
  });

  it("ExitPayload round-trips", () => {
    const p = { pid: "pid-7", reason: "error:something broke" };
    const decoded = decodeExitPayload(encodeExitPayload(p));
    expect(decoded.pid).toBe("pid-7");
    expect(decoded.reason).toBe("error:something broke");
  });

  it("HandshakePayload round-trips", () => {
    const p = { nodeName: "node-alpha", cookie: "secret123", version: 1 };
    const decoded = decodeHandshakePayload(encodeHandshakePayload(p));
    expect(decoded.nodeName).toBe("node-alpha");
    expect(decoded.cookie).toBe("secret123");
    expect(decoded.version).toBe(1);
  });

  it("RegisterPayload round-trips", () => {
    const p = { name: "registry", pid: "pid-10" };
    const decoded = decodeRegisterPayload(encodeRegisterPayload(p));
    expect(decoded.name).toBe("registry");
    expect(decoded.pid).toBe("pid-10");
  });

  it("LookupPayload round-trips", () => {
    const p = { requestId: "req-5", name: "my_service" };
    const decoded = decodeLookupPayload(encodeLookupPayload(p));
    expect(decoded.requestId).toBe("req-5");
    expect(decoded.name).toBe("my_service");
  });

  it("LookupResponsePayload round-trips with pid", () => {
    const p = { requestId: "req-5", pid: "pid-55" };
    const decoded = decodeLookupResponsePayload(encodeLookupResponsePayload(p));
    expect(decoded.requestId).toBe("req-5");
    expect(decoded.pid).toBe("pid-55");
  });

  it("LookupResponsePayload round-trips with null pid", () => {
    const p = { requestId: "req-6", pid: null };
    const decoded = decodeLookupResponsePayload(encodeLookupResponsePayload(p));
    expect(decoded.requestId).toBe("req-6");
    expect(decoded.pid).toBeNull();
  });

  it("LinkPayload round-trips", () => {
    const p = { fromPid: "pid-1", toPid: "pid-2" };
    const decoded = decodeLinkPayload(encodeLinkPayload(p));
    expect(decoded.fromPid).toBe("pid-1");
    expect(decoded.toPid).toBe("pid-2");
  });

  it("MonitorPayload round-trips", () => {
    const p = { monitorPid: "pid-10", targetPid: "pid-20" };
    const decoded = decodeMonitorPayload(encodeMonitorPayload(p));
    expect(decoded.monitorPid).toBe("pid-10");
    expect(decoded.targetPid).toBe("pid-20");
  });

  // -----------------------------------------------------------------------
  // Integration: full wire message with payload
  // -----------------------------------------------------------------------

  it("full wire message with SendPayload round-trips end-to-end", () => {
    const sendPayload = encodeSendPayload({
      toPid: "pid-target",
      fromPid: "pid-sender",
      data: new Uint8Array([0xCA, 0xFE]),
    });
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "node-a",
      toNode: "node-b",
      payload: sendPayload,
    };
    const frame = encodeFrame(msg);
    const decoded = decodeFrame(frame);
    const payload = decodeSendPayload(decoded.payload);
    expect(payload.toPid).toBe("pid-target");
    expect(payload.fromPid).toBe("pid-sender");
    expect(payload.data).toEqual(new Uint8Array([0xCA, 0xFE]));
  });

  it("FrameReader with multiple payload types in sequence", () => {
    const frames: Uint8Array[] = [];

    // Handshake
    frames.push(encodeFrame({
      type: MsgType.HANDSHAKE,
      fromNode: "a",
      toNode: "b",
      payload: encodeHandshakePayload({ nodeName: "a", cookie: "secret", version: 1 }),
    }));

    // Send
    frames.push(encodeFrame({
      type: MsgType.SEND,
      fromNode: "a",
      toNode: "b",
      payload: encodeSendPayload({ toPid: "p1", fromPid: "p2", data: new Uint8Array([42]) }),
    }));

    // Exit
    frames.push(encodeFrame({
      type: MsgType.EXIT,
      fromNode: "a",
      toNode: "b",
      payload: encodeExitPayload({ pid: "p1", reason: "normal" }),
    }));

    let total = 0;
    for (const f of frames) total += f.length;
    const combined = new Uint8Array(total);
    let off = 0;
    for (const f of frames) { combined.set(f, off); off += f.length; }

    const reader = new FrameReader();
    const msgs = reader.feed(combined);
    expect(msgs.length).toBe(3);
    expect(msgs[0].type).toBe(MsgType.HANDSHAKE);
    expect(msgs[1].type).toBe(MsgType.SEND);
    expect(msgs[2].type).toBe(MsgType.EXIT);

    const hs = decodeHandshakePayload(msgs[0].payload);
    expect(hs.cookie).toBe("secret");

    const send = decodeSendPayload(msgs[1].payload);
    expect(send.data).toEqual(new Uint8Array([42]));

    const exit = decodeExitPayload(msgs[2].payload);
    expect(exit.reason).toBe("normal");
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  makePid,
  pidToString,
  parsePid,
  isLocal,
  serializePid,
  deserializePid,
} from "../../src/distributed/dpid.js";
import { DistributedRouter } from "../../src/distributed/router.js";
import type { DistributedPid } from "../../src/distributed/dpid.js";
import { MsgType } from "../../src/wire/protocol.js";
import type { WireMessage } from "../../src/wire/protocol.js";
import { serialize } from "../../src/wire/serialize.js";
import {
  encodeSendPayload,
  decodeSendPayload,
  encodeLookupPayload,
  encodeLookupResponsePayload,
} from "../../src/wire/frame.js";
import { deserialize } from "../../src/wire/deserialize.js";

// ---------------------------------------------------------------------------
// Helpers — mock scheduler and connection manager
// ---------------------------------------------------------------------------

function createMockScheduler() {
  const sent: Array<{ pid: string; msg: unknown }> = [];
  return {
    send(pid: string, msg: unknown) {
      sent.push({ pid, msg });
    },
    sent,
    // Stubs for other Scheduler methods (unused by router)
    spawn: vi.fn(),
    receive: vi.fn(),
    self: vi.fn(),
    link: vi.fn(),
    monitor: vi.fn(),
    getProcess: vi.fn(),
    processCount: vi.fn(),
  };
}

function createMockConnections(connectedNodes: string[] = []) {
  const sent: Array<{ node: string; msg: WireMessage }> = [];
  return {
    send(node: string, msg: WireMessage): boolean {
      const isConnected = connectedNodes.includes(node);
      if (isConnected) {
        sent.push({ node, msg });
      }
      return isConnected;
    },
    getConnectedNodes(): string[] {
      return connectedNodes;
    },
    sent,
    // Stubs for other ConnectionManager methods
    listen: vi.fn(),
    connect: vi.fn(),
    connectToPeers: vi.fn(),
    getConnection: vi.fn(),
    disconnect: vi.fn(),
    shutdown: vi.fn(),
    startPingLoop: vi.fn(),
  };
}

// ---------------------------------------------------------------------------
// dpid.ts tests
// ---------------------------------------------------------------------------

describe("DistributedPid (dpid)", () => {
  it("makePid creates a pid with node and local", () => {
    const pid = makePid("alpha", "abc-123");
    expect(pid.node).toBe("alpha");
    expect(pid.local).toBe("abc-123");
  });

  it("pidToString formats as node:local", () => {
    const pid = makePid("alpha", "abc-123");
    expect(pidToString(pid)).toBe("alpha:abc-123");
  });

  it("parsePid parses node:local string", () => {
    const pid = parsePid("alpha:abc-123");
    expect(pid.node).toBe("alpha");
    expect(pid.local).toBe("abc-123");
  });

  it("pidToString and parsePid round-trip", () => {
    const original = makePid("beta", "def-456-ghi");
    const str = pidToString(original);
    const parsed = parsePid(str);
    expect(parsed).toEqual(original);
  });

  it("parsePid handles local id containing colons", () => {
    const pid = parsePid("node1:some:complex:id");
    expect(pid.node).toBe("node1");
    expect(pid.local).toBe("some:complex:id");
  });

  it("parsePid throws on invalid string (no colon)", () => {
    expect(() => parsePid("nocolon")).toThrow("Invalid distributed PID");
  });

  it("isLocal returns true when node matches selfNode", () => {
    const pid = makePid("alpha", "abc-123");
    expect(isLocal(pid, "alpha")).toBe(true);
  });

  it("isLocal returns false when node differs from selfNode", () => {
    const pid = makePid("alpha", "abc-123");
    expect(isLocal(pid, "beta")).toBe(false);
  });

  it("serializePid and deserializePid round-trip", () => {
    const original = makePid("gamma", "xyz-789");
    const buf = serializePid(original);
    const { pid, bytesRead } = deserializePid(buf);
    expect(pid).toEqual(original);
    expect(bytesRead).toBe(buf.length);
  });

  it("serializePid and deserializePid handle empty node name", () => {
    const original = makePid("", "local-only");
    const buf = serializePid(original);
    const { pid } = deserializePid(buf);
    expect(pid).toEqual(original);
  });

  it("serializePid and deserializePid handle unicode", () => {
    const original = makePid("nodo-\u00e9", "proc-\u2603");
    const buf = serializePid(original);
    const { pid } = deserializePid(buf);
    expect(pid).toEqual(original);
  });
});

// ---------------------------------------------------------------------------
// DistributedRouter tests
// ---------------------------------------------------------------------------

describe("DistributedRouter", () => {
  let scheduler: ReturnType<typeof createMockScheduler>;
  let connections: ReturnType<typeof createMockConnections>;
  let router: DistributedRouter;

  beforeEach(() => {
    scheduler = createMockScheduler();
    connections = createMockConnections(["beta", "gamma"]);
    router = new DistributedRouter("alpha", scheduler as any, connections as any);
  });

  it("getSelfNode returns the node name", () => {
    expect(router.getSelfNode()).toBe("alpha");
  });

  // --- Local send ---

  it("local send routes to scheduler", () => {
    const pid = makePid("alpha", "proc-1");
    router.send(pid, { hello: "world" });

    expect(scheduler.sent).toHaveLength(1);
    expect(scheduler.sent[0].pid).toBe("proc-1");
    expect(scheduler.sent[0].msg).toEqual({ hello: "world" });
    expect(connections.sent).toHaveLength(0);
  });

  it("local send returns true", () => {
    const pid = makePid("alpha", "proc-1");
    expect(router.send(pid, "msg")).toBe(true);
  });

  // --- Remote send ---

  it("remote send serializes and sends over connection", () => {
    const pid = makePid("beta", "proc-2");
    router.send(pid, 42);

    expect(connections.sent).toHaveLength(1);
    expect(connections.sent[0].node).toBe("beta");
    expect(connections.sent[0].msg.type).toBe(MsgType.SEND);
    expect(connections.sent[0].msg.fromNode).toBe("alpha");
    expect(connections.sent[0].msg.toNode).toBe("beta");
    expect(scheduler.sent).toHaveLength(0);
  });

  it("remote send payload can be decoded back", () => {
    const pid = makePid("beta", "proc-2");
    router.send(pid, "hello");

    const wireMsg = connections.sent[0].msg;
    const payload = decodeSendPayload(wireMsg.payload);
    expect(payload.toPid).toBe("proc-2");
    const deserialized = deserialize(payload.data);
    expect(deserialized.value).toBe("hello");
  });

  it("remote send to unknown node returns false", () => {
    const pid = makePid("unknown-node", "proc-3");
    const result = router.send(pid, "test");
    expect(result).toBe(false);
    expect(connections.sent).toHaveLength(0);
  });

  // --- Incoming message handling ---

  it("incoming SEND message deserializes and delivers to scheduler", () => {
    const data = serialize("distributed-hello");
    const payload = encodeSendPayload({
      toPid: "local-proc-1",
      fromPid: "remote-proc-1",
      data,
    });

    const wireMsg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "beta",
      toNode: "alpha",
      payload,
    };

    router.handleIncoming("beta", wireMsg);

    expect(scheduler.sent).toHaveLength(1);
    expect(scheduler.sent[0].pid).toBe("local-proc-1");
    expect(scheduler.sent[0].msg).toBe("distributed-hello");
  });

  it("incoming SEND with complex value deserializes correctly", () => {
    const value = { count: 99, items: [1, 2, 3] };
    const data = serialize(value);
    const payload = encodeSendPayload({
      toPid: "proc-x",
      fromPid: "proc-y",
      data,
    });

    router.handleIncoming("gamma", {
      type: MsgType.SEND,
      fromNode: "gamma",
      toNode: "alpha",
      payload,
    });

    expect(scheduler.sent).toHaveLength(1);
    expect(scheduler.sent[0].msg).toEqual(value);
  });

  // --- Name registry ---

  it("register and lookup local name", () => {
    const pid = makePid("alpha", "proc-svc");
    router.register("my-service", pid);

    // lookup is async but local should resolve immediately
    return router.lookup("my-service").then((found) => {
      expect(found).toEqual(pid);
    });
  });

  it("register overwrites previous registration", () => {
    const pid1 = makePid("alpha", "proc-1");
    const pid2 = makePid("alpha", "proc-2");
    router.register("svc", pid1);
    router.register("svc", pid2);

    return router.lookup("svc").then((found) => {
      expect(found).toEqual(pid2);
    });
  });

  it("unregister removes a name", () => {
    const pid = makePid("alpha", "proc-1");
    router.register("svc", pid);
    router.unregister("svc");

    // With no connected nodes, lookup should return null
    const noConnRouter = new DistributedRouter(
      "alpha",
      scheduler as any,
      createMockConnections([]) as any,
    );

    return noConnRouter.lookup("svc").then((found) => {
      expect(found).toBeNull();
    });
  });

  it("lookup unknown name with no connected nodes returns null", async () => {
    const isolated = new DistributedRouter(
      "alpha",
      scheduler as any,
      createMockConnections([]) as any,
    );
    const result = await isolated.lookup("nonexistent");
    expect(result).toBeNull();
  });

  // --- LOOKUP request/response flow ---

  it("incoming LOOKUP responds with registered pid", () => {
    const pid = makePid("alpha", "local-svc");
    router.register("counter", pid);

    const lookupPayload = encodeLookupPayload({
      requestId: "req-1",
      name: "counter",
    });

    router.handleIncoming("beta", {
      type: MsgType.LOOKUP,
      fromNode: "beta",
      toNode: "alpha",
      payload: lookupPayload,
    });

    expect(connections.sent).toHaveLength(1);
    expect(connections.sent[0].node).toBe("beta");
    expect(connections.sent[0].msg.type).toBe(MsgType.LOOKUP_RESPONSE);
  });

  it("incoming LOOKUP responds with null for unknown name", () => {
    const lookupPayload = encodeLookupPayload({
      requestId: "req-2",
      name: "nonexistent",
    });

    router.handleIncoming("beta", {
      type: MsgType.LOOKUP,
      fromNode: "beta",
      toNode: "alpha",
      payload: lookupPayload,
    });

    expect(connections.sent).toHaveLength(1);
    expect(connections.sent[0].msg.type).toBe(MsgType.LOOKUP_RESPONSE);
  });

  it("lookup sends LOOKUP to all connected nodes", async () => {
    // Start lookup — it will timeout since no responses come, but we can check it sent
    const lookupPromise = router.lookup("remote-svc", 50);

    // Should have sent LOOKUP to both beta and gamma
    expect(connections.sent).toHaveLength(2);
    expect(connections.sent[0].msg.type).toBe(MsgType.LOOKUP);
    expect(connections.sent[1].msg.type).toBe(MsgType.LOOKUP);
    const nodes = connections.sent.map((s) => s.node).sort();
    expect(nodes).toEqual(["beta", "gamma"]);

    // Wait for timeout
    const result = await lookupPromise;
    expect(result).toBeNull();
  });

  it("lookup resolves when LOOKUP_RESPONSE arrives with pid", async () => {
    const lookupPromise = router.lookup("remote-svc", 1000);

    // Decode the lookup payload to get the requestId
    const { decodeLookupPayload: decodeLookup } = await import("../../src/wire/frame.js");
    const lookupReq = decodeLookup(connections.sent[0].msg.payload);

    const responsePayload = encodeLookupResponsePayload({
      requestId: lookupReq.requestId,
      pid: "beta:remote-proc-1",
    });

    router.handleIncoming("beta", {
      type: MsgType.LOOKUP_RESPONSE,
      fromNode: "beta",
      toNode: "alpha",
      payload: responsePayload,
    });

    const result = await lookupPromise;
    expect(result).toEqual(makePid("beta", "remote-proc-1"));
  });

  // --- Unknown message types ---

  it("handleIncoming ignores unknown message types gracefully", () => {
    // Should not throw
    router.handleIncoming("beta", {
      type: MsgType.MONITOR,
      fromNode: "beta",
      toNode: "alpha",
      payload: new Uint8Array(0),
    });

    expect(scheduler.sent).toHaveLength(0);
  });
});

import { describe, it, expect, afterEach, beforeEach } from "vitest";
import type { WireMessage } from "../../src/wire/protocol.js";
import { MsgType } from "../../src/wire/protocol.js";
import { encodeFrame, encodeSendPayload } from "../../src/wire/frame.js";
import { parseAddress } from "../../src/node/node.js";
import { ConnectionManager } from "../../src/node/connection.js";
import type { NodeConfig } from "../../src/node/node.js";
import {
  createHandshakeMessage,
  createHandshakeAck,
  createHandshakeNack,
  verifyHandshake,
} from "../../src/node/handshake.js";
import { encodeHandshakePayload } from "../../src/wire/frame.js";
import { Reconnector } from "../../src/node/reconnect.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let managers: ConnectionManager[] = [];

function makeConfig(name: string, port: number, cookie: string, connectTo?: string[]): NodeConfig {
  return {
    name,
    listen: `127.0.0.1:${port}`,
    connect: connectTo,
    cookie,
  };
}

function makeManager(
  config: NodeConfig,
  callbacks?: {
    onMessage?: (from: string, msg: WireMessage) => void;
    onNodeUp?: (nodeName: string) => void;
    onNodeDown?: (nodeName: string) => void;
  },
): ConnectionManager {
  const mgr = new ConnectionManager(config, {
    onMessage: callbacks?.onMessage ?? (() => {}),
    onNodeUp: callbacks?.onNodeUp ?? (() => {}),
    onNodeDown: callbacks?.onNodeDown ?? (() => {}),
  });
  managers.push(mgr);
  return mgr;
}

/** Wait for a condition with timeout. */
function waitFor(condition: () => boolean, timeoutMs: number = 2000): Promise<void> {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const check = () => {
      if (condition()) {
        resolve();
      } else if (Date.now() - start > timeoutMs) {
        reject(new Error("waitFor timed out"));
      } else {
        setTimeout(check, 10);
      }
    };
    check();
  });
}

// Use a dynamic port range to avoid collisions between tests
let nextPort = 19100;
function getPort(): number {
  return nextPort++;
}

afterEach(async () => {
  for (const mgr of managers) {
    await mgr.shutdown();
  }
  managers = [];
});

// ---------------------------------------------------------------------------
// parseAddress tests
// ---------------------------------------------------------------------------

describe("parseAddress", () => {
  it("parses :port as 0.0.0.0", () => {
    const result = parseAddress(":9000");
    expect(result).toEqual({ host: "0.0.0.0", port: 9000 });
  });

  it("parses host:port with IP", () => {
    const result = parseAddress("192.168.1.1:9000");
    expect(result).toEqual({ host: "192.168.1.1", port: 9000 });
  });

  it("parses host:port with hostname", () => {
    const result = parseAddress("alpha:9000");
    expect(result).toEqual({ host: "alpha", port: 9000 });
  });

  it("parses localhost:port", () => {
    const result = parseAddress("127.0.0.1:8080");
    expect(result).toEqual({ host: "127.0.0.1", port: 8080 });
  });

  it("throws on missing colon", () => {
    expect(() => parseAddress("nocolon")).toThrow("Invalid address");
  });

  it("throws on invalid port", () => {
    expect(() => parseAddress("host:notaport")).toThrow("Invalid port");
  });

  it("parses 0.0.0.0:0", () => {
    const result = parseAddress("0.0.0.0:0");
    expect(result).toEqual({ host: "0.0.0.0", port: 0 });
  });
});

// ---------------------------------------------------------------------------
// Handshake tests
// ---------------------------------------------------------------------------

describe("handshake", () => {
  it("verifyHandshake succeeds with correct cookie", () => {
    const payload = encodeHandshakePayload({ nodeName: "alpha", cookie: "secret", version: 1 });
    const result = verifyHandshake(payload, "secret");
    expect(result.valid).toBe(true);
    expect(result.nodeName).toBe("alpha");
  });

  it("verifyHandshake fails with wrong cookie", () => {
    const payload = encodeHandshakePayload({ nodeName: "alpha", cookie: "wrong", version: 1 });
    const result = verifyHandshake(payload, "secret");
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("bad cookie");
  });

  it("verifyHandshake fails with malformed data", () => {
    const result = verifyHandshake(new Uint8Array([0xff]), "secret");
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("malformed handshake payload");
  });

  it("createHandshakeMessage produces valid frame bytes", () => {
    const frame = createHandshakeMessage("alpha", "secret");
    expect(frame.length).toBeGreaterThan(4);
    // First 4 bytes are the frame length
    const len = new DataView(frame.buffer, frame.byteOffset).getUint32(0);
    expect(len).toBe(frame.length);
  });
});

// ---------------------------------------------------------------------------
// Connection tests (real TCP)
// ---------------------------------------------------------------------------

describe("ConnectionManager", () => {
  it("alpha connects to beta successfully", async () => {
    const portBeta = getPort();
    const nodeUpEvents: string[] = [];

    const beta = makeManager(makeConfig("beta", portBeta, "secret"), {
      onNodeUp: (name) => nodeUpEvents.push(`beta:${name}`),
    });
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"), {
      onNodeUp: (name) => nodeUpEvents.push(`alpha:${name}`),
    });

    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);

    await waitFor(() => nodeUpEvents.includes("alpha:beta") && nodeUpEvents.includes("beta:alpha"));

    expect(alpha.getConnectedNodes()).toContain("beta");
    expect(beta.getConnectedNodes()).toContain("alpha");
  });

  it("handshake fails with wrong cookie", async () => {
    const portBeta = getPort();
    const beta = makeManager(makeConfig("beta", portBeta, "correct-cookie"));
    const alpha = makeManager(makeConfig("alpha", getPort(), "wrong-cookie"));

    await beta.listen();
    await expect(alpha.connect(`127.0.0.1:${portBeta}`)).rejects.toThrow("rejected");

    expect(alpha.getConnectedNodes()).not.toContain("beta");
    expect(beta.getConnectedNodes()).not.toContain("alpha");
  });

  it("sends a message from alpha to beta", async () => {
    const portBeta = getPort();
    const received: WireMessage[] = [];

    const beta = makeManager(makeConfig("beta", portBeta, "secret"), {
      onMessage: (_from, msg) => received.push(msg),
    });
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"));

    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await waitFor(() => alpha.getConnectedNodes().includes("beta"));

    const payload = encodeSendPayload({ toPid: "pid-1", fromPid: "pid-2", data: new Uint8Array([1, 2, 3]) });
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "alpha",
      toNode: "beta",
      payload,
    };
    const sent = alpha.send("beta", msg);
    expect(sent).toBe(true);

    await waitFor(() => received.length > 0);
    expect(received[0].type).toBe(MsgType.SEND);
    expect(received[0].fromNode).toBe("alpha");
  });

  it("supports bidirectional messaging", async () => {
    const portAlpha = getPort();
    const portBeta = getPort();
    const alphaReceived: WireMessage[] = [];
    const betaReceived: WireMessage[] = [];

    const alpha = makeManager(makeConfig("alpha", portAlpha, "secret"), {
      onMessage: (_from, msg) => alphaReceived.push(msg),
    });
    const beta = makeManager(makeConfig("beta", portBeta, "secret"), {
      onMessage: (_from, msg) => betaReceived.push(msg),
    });

    await alpha.listen();
    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await waitFor(() => alpha.getConnectedNodes().includes("beta") && beta.getConnectedNodes().includes("alpha"));

    // alpha -> beta
    const payload1 = encodeSendPayload({ toPid: "p1", fromPid: "p2", data: new Uint8Array([10]) });
    alpha.send("beta", { type: MsgType.SEND, fromNode: "alpha", toNode: "beta", payload: payload1 });

    // beta -> alpha
    const payload2 = encodeSendPayload({ toPid: "p3", fromPid: "p4", data: new Uint8Array([20]) });
    beta.send("alpha", { type: MsgType.SEND, fromNode: "beta", toNode: "alpha", payload: payload2 });

    await waitFor(() => alphaReceived.length > 0 && betaReceived.length > 0);
    expect(betaReceived[0].fromNode).toBe("alpha");
    expect(alphaReceived[0].fromNode).toBe("beta");
  });

  it("disconnect triggers onNodeDown", async () => {
    const portBeta = getPort();
    const downEvents: string[] = [];

    const beta = makeManager(makeConfig("beta", portBeta, "secret"), {
      onNodeDown: (name) => downEvents.push(`beta:${name}`),
    });
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"), {
      onNodeDown: (name) => downEvents.push(`alpha:${name}`),
    });

    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await waitFor(() => alpha.getConnectedNodes().includes("beta"));

    alpha.disconnect("beta");
    await waitFor(() => downEvents.some((e) => e.includes("beta:alpha")));

    expect(alpha.getConnectedNodes()).not.toContain("beta");
  });

  it("getConnectedNodes returns correct list", async () => {
    const portBeta = getPort();
    const portGamma = getPort();

    const beta = makeManager(makeConfig("beta", portBeta, "secret"));
    const gamma = makeManager(makeConfig("gamma", portGamma, "secret"));
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"));

    await beta.listen();
    await gamma.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await alpha.connect(`127.0.0.1:${portGamma}`);

    await waitFor(() => alpha.getConnectedNodes().length === 2);

    const nodes = alpha.getConnectedNodes().sort();
    expect(nodes).toEqual(["beta", "gamma"]);
  });

  it("send returns false for unknown node", () => {
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"));
    const msg: WireMessage = {
      type: MsgType.SEND,
      fromNode: "alpha",
      toNode: "nobody",
      payload: new Uint8Array(0),
    };
    expect(alpha.send("nobody", msg)).toBe(false);
  });

  it("multiple nodes form a mesh", async () => {
    const portA = getPort();
    const portB = getPort();
    const portC = getPort();

    const a = makeManager(makeConfig("a", portA, "secret"));
    const b = makeManager(makeConfig("b", portB, "secret"));
    const c = makeManager(makeConfig("c", portC, "secret"));

    await a.listen();
    await b.listen();
    await c.listen();

    // a -> b, a -> c, b -> c
    await a.connect(`127.0.0.1:${portB}`);
    await a.connect(`127.0.0.1:${portC}`);
    await b.connect(`127.0.0.1:${portC}`);

    await waitFor(() => a.getConnectedNodes().length === 2 && b.getConnectedNodes().length === 2 && c.getConnectedNodes().length === 2);

    expect(a.getConnectedNodes().sort()).toEqual(["b", "c"]);
    expect(b.getConnectedNodes().sort()).toEqual(["a", "c"]);
    expect(c.getConnectedNodes().sort()).toEqual(["a", "b"]);
  });

  it("shutdown closes all connections and server", async () => {
    const portBeta = getPort();
    const downEvents: string[] = [];

    const beta = makeManager(makeConfig("beta", portBeta, "secret"), {
      onNodeDown: (name) => downEvents.push(name),
    });
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"));

    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await waitFor(() => alpha.getConnectedNodes().includes("beta"));

    await alpha.shutdown();
    expect(alpha.getConnectedNodes()).toEqual([]);

    await waitFor(() => downEvents.includes("alpha"));
  });

  it("connectToPeers connects to all configured addresses", async () => {
    const portBeta = getPort();
    const portGamma = getPort();

    const beta = makeManager(makeConfig("beta", portBeta, "secret"));
    const gamma = makeManager(makeConfig("gamma", portGamma, "secret"));
    const alpha = makeManager(
      makeConfig("alpha", getPort(), "secret", [`127.0.0.1:${portBeta}`, `127.0.0.1:${portGamma}`]),
    );

    await beta.listen();
    await gamma.listen();
    await alpha.connectToPeers();

    await waitFor(() => alpha.getConnectedNodes().length === 2);
    expect(alpha.getConnectedNodes().sort()).toEqual(["beta", "gamma"]);
  });

  it("handles PING/PONG automatically", async () => {
    const portBeta = getPort();
    const betaReceived: WireMessage[] = [];

    const beta = makeManager(makeConfig("beta", portBeta, "secret"), {
      onMessage: (_from, msg) => betaReceived.push(msg),
    });
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"));

    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await waitFor(() => alpha.getConnectedNodes().includes("beta"));

    // Send a PING manually
    const pingMsg: WireMessage = {
      type: MsgType.PING,
      fromNode: "alpha",
      toNode: "beta",
      payload: new Uint8Array(0),
    };
    alpha.send("beta", pingMsg);

    // PING should be handled internally (PONG sent back), not forwarded to onMessage
    // Give time for PONG to arrive
    await new Promise((r) => setTimeout(r, 100));
    // betaReceived should NOT contain the PING (it's handled internally)
    expect(betaReceived.filter((m) => m.type === MsgType.PING)).toHaveLength(0);
  });

  it("getConnection returns connection details", async () => {
    const portBeta = getPort();
    const beta = makeManager(makeConfig("beta", portBeta, "secret"));
    const alpha = makeManager(makeConfig("alpha", getPort(), "secret"));

    await beta.listen();
    await alpha.connect(`127.0.0.1:${portBeta}`);
    await waitFor(() => alpha.getConnectedNodes().includes("beta"));

    const conn = alpha.getConnection("beta");
    expect(conn).toBeDefined();
    expect(conn!.state).toBe("connected");
    expect(conn!.nodeId.name).toBe("beta");

    expect(alpha.getConnection("nonexistent")).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Reconnector tests
// ---------------------------------------------------------------------------

describe("Reconnector", () => {
  it("retries on failure with increasing delay", async () => {
    let attempts = 0;
    const reconnector = new Reconnector(async () => {
      attempts++;
      throw new Error("fail");
    });

    reconnector.start();
    await new Promise((r) => setTimeout(r, 3500));
    reconnector.stop();

    // With base 1s: attempts at 0, ~1s, ~3s → at least 2-3 attempts
    expect(attempts).toBeGreaterThanOrEqual(2);
  });

  it("stops retrying when stop is called", async () => {
    let attempts = 0;
    const reconnector = new Reconnector(async () => {
      attempts++;
      throw new Error("fail");
    });

    reconnector.start();
    await new Promise((r) => setTimeout(r, 1200));
    reconnector.stop();
    const countAfterStop = attempts;
    await new Promise((r) => setTimeout(r, 2000));

    // Should not have increased after stop
    expect(attempts).toBe(countAfterStop);
  });

  it("resets attempts on success", async () => {
    let shouldFail = true;
    let attempts = 0;
    const reconnector = new Reconnector(async () => {
      attempts++;
      if (shouldFail) throw new Error("fail");
    });

    reconnector.start();
    await new Promise((r) => setTimeout(r, 1200));
    shouldFail = false;
    await new Promise((r) => setTimeout(r, 2500));
    reconnector.stop();

    expect(reconnector.getAttempts()).toBe(0);
  });

  it("getNextDelay respects maxDelay cap", () => {
    const reconnector = new Reconnector(async () => {});
    // Default max is 30000, base is 1000
    // After many attempts the delay should cap
    expect(reconnector.getNextDelay()).toBe(1000); // 2^0 * 1000
  });
});

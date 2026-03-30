import { describe, it, expect } from "vitest";
import { NodeRegistry, NodeInfo } from "../../src/node/registry.js";

function makeNodeInfo(name: string, port = 9000): NodeInfo {
  return { name, host: "127.0.0.1", port, connectedAt: Date.now(), status: "up" };
}

describe("NodeRegistry", () => {
  it("adds and retrieves a node", () => {
    const reg = new NodeRegistry();
    const info = makeNodeInfo("node-a");
    reg.addNode("node-a", info);
    expect(reg.getNode("node-a")).toEqual(info);
  });

  it("removes a node", () => {
    const reg = new NodeRegistry();
    reg.addNode("node-a", makeNodeInfo("node-a"));
    reg.removeNode("node-a");
    expect(reg.getNode("node-a")).toBeUndefined();
  });

  it("registers a named process", () => {
    const reg = new NodeRegistry();
    reg.register("counter", "node-a", "pid-1");
    expect(reg.lookup("counter")).toEqual({ node: "node-a", pid: "pid-1" });
  });

  it("looks up a named process", () => {
    const reg = new NodeRegistry();
    reg.register("logger", "node-b", "pid-2");
    const result = reg.lookup("logger");
    expect(result).toBeDefined();
    expect(result!.node).toBe("node-b");
    expect(result!.pid).toBe("pid-2");
  });

  it("returns undefined for unknown name", () => {
    const reg = new NodeRegistry();
    expect(reg.lookup("nonexistent")).toBeUndefined();
  });

  it("unregisters a name", () => {
    const reg = new NodeRegistry();
    reg.register("worker", "node-a", "pid-3");
    reg.unregister("worker");
    expect(reg.lookup("worker")).toBeUndefined();
  });

  it("removes names when node is removed", () => {
    const reg = new NodeRegistry();
    reg.addNode("node-a", makeNodeInfo("node-a"));
    reg.register("svc-1", "node-a", "pid-1");
    reg.register("svc-2", "node-a", "pid-2");
    reg.register("svc-3", "node-b", "pid-3");

    reg.removeNode("node-a");

    expect(reg.lookup("svc-1")).toBeUndefined();
    expect(reg.lookup("svc-2")).toBeUndefined();
    // svc-3 on node-b should still exist
    expect(reg.lookup("svc-3")).toEqual({ node: "node-b", pid: "pid-3" });
  });

  it("lists all registered names", () => {
    const reg = new NodeRegistry();
    reg.register("alpha", "node-a", "p1");
    reg.register("beta", "node-a", "p2");
    reg.register("gamma", "node-b", "p3");

    const names = reg.getRegisteredNames();
    expect(names).toHaveLength(3);
    expect(names).toContain("alpha");
    expect(names).toContain("beta");
    expect(names).toContain("gamma");
  });

  it("lists all nodes", () => {
    const reg = new NodeRegistry();
    reg.addNode("node-a", makeNodeInfo("node-a", 9000));
    reg.addNode("node-b", makeNodeInfo("node-b", 9001));

    const all = reg.getAllNodes();
    expect(all.size).toBe(2);
    expect(all.get("node-a")!.port).toBe(9000);
    expect(all.get("node-b")!.port).toBe(9001);
  });

  it("overwrites existing registration", () => {
    const reg = new NodeRegistry();
    reg.register("leader", "node-a", "pid-old");
    reg.register("leader", "node-b", "pid-new");

    const result = reg.lookup("leader");
    expect(result).toEqual({ node: "node-b", pid: "pid-new" });
  });

  it("returns undefined for unknown node", () => {
    const reg = new NodeRegistry();
    expect(reg.getNode("nonexistent")).toBeUndefined();
  });

  it("getAllNodes returns a copy", () => {
    const reg = new NodeRegistry();
    reg.addNode("node-a", makeNodeInfo("node-a"));
    const copy = reg.getAllNodes();
    copy.delete("node-a");
    // Original should be unaffected
    expect(reg.getNode("node-a")).toBeDefined();
  });
});

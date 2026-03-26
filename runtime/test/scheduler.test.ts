import { describe, it, expect } from "vitest";
import { Scheduler } from "../src/scheduler.js";
import type { ProcessContext } from "../src/process.js";

function delay(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

describe("Scheduler", () => {
  it("spawns a process and returns a pid", () => {
    const s = new Scheduler();
    const pid = s.spawn(async () => {});
    expect(typeof pid).toBe("string");
    expect(pid.length).toBeGreaterThan(0);
  });

  it("tracks process count", () => {
    const s = new Scheduler();
    expect(s.processCount()).toBe(0);
    s.spawn(async () => {});
    expect(s.processCount()).toBe(1);
    s.spawn(async () => {});
    expect(s.processCount()).toBe(2);
  });

  it("process completes normally with state done", async () => {
    const s = new Scheduler();
    const pid = s.spawn(async () => {});
    await delay(10);
    const ctx = s.getProcess(pid);
    expect(ctx?.state).toBe("done");
  });

  it("process that throws becomes failed", async () => {
    const s = new Scheduler();
    const pid = s.spawn(async () => {
      throw new Error("boom");
    });
    await delay(10);
    const ctx = s.getProcess(pid);
    expect(ctx?.state).toBe("failed");
    expect(ctx?.crashReason?._tag).toBe("Error");
    if (ctx?.crashReason?._tag === "Error") {
      expect(ctx.crashReason.message).toBe("boom");
    }
  });

  it("send and receive between processes", async () => {
    const s = new Scheduler();
    let received: unknown = null;

    const pid1 = s.spawn(async (ctx: ProcessContext) => {
      received = await ctx.mailbox.receive();
    });

    s.send(pid1, "hello");
    await delay(10);
    expect(received).toBe("hello");
  });

  it("bidirectional send/receive", async () => {
    const s = new Scheduler();
    let result: unknown = null;

    const pid1 = s.spawn(async (ctx: ProcessContext) => {
      const msg = await ctx.mailbox.receive() as { from: string; body: string };
      s.send(msg.from, { body: "pong" });
    });

    const pid2 = s.spawn(async (ctx: ProcessContext) => {
      s.send(pid1, { from: ctx.id, body: "ping" });
      const reply = await ctx.mailbox.receive() as { body: string };
      result = reply.body;
    });

    await delay(20);
    expect(result).toBe("pong");
  });

  it("spawns 100 processes", async () => {
    const s = new Scheduler();
    const pids: string[] = [];
    for (let i = 0; i < 100; i++) {
      pids.push(s.spawn(async () => {}));
    }
    expect(pids.length).toBe(100);
    expect(s.processCount()).toBe(100);
    await delay(50);
    // All should be done
    for (const pid of pids) {
      expect(s.getProcess(pid)?.state).toBe("done");
    }
  });

  it("spawns 1000 processes", async () => {
    const s = new Scheduler();
    let completedCount = 0;
    for (let i = 0; i < 1000; i++) {
      s.spawn(async () => {
        completedCount++;
      });
    }
    expect(s.processCount()).toBe(1000);
    await delay(100);
    expect(completedCount).toBe(1000);
  });

  it("link propagates crash", async () => {
    const s = new Scheduler();

    const pid1 = s.spawn(async (ctx: ProcessContext) => {
      // Wait for messages forever
      await ctx.mailbox.receive();
    });

    const pid2 = s.spawn(async () => {
      await delay(10);
      throw new Error("crash");
    });

    s.link(pid1, pid2);

    await delay(50);
    const ctx1 = s.getProcess(pid1);
    const ctx2 = s.getProcess(pid2);
    expect(ctx2?.state).toBe("failed");
    expect(ctx1?.state).toBe("failed");
    expect(ctx1?.crashReason?._tag).toBe("LinkedCrash");
  });

  it("monitor sends DOWN message on crash", async () => {
    const s = new Scheduler();
    let downMsg: unknown = null;

    const watcher = s.spawn(async (ctx: ProcessContext) => {
      downMsg = await ctx.mailbox.receive();
    });

    const target = s.spawn(async () => {
      await delay(10);
      throw new Error("oops");
    });

    s.monitor(watcher, target);

    await delay(50);
    expect(downMsg).not.toBeNull();
    expect((downMsg as { _type: string })._type).toBe("DOWN");
  });

  it("getProcess returns undefined for unknown pid", () => {
    const s = new Scheduler();
    expect(s.getProcess("nonexistent")).toBeUndefined();
  });
});

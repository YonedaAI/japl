import type { ProcessId } from "./types.js";
import type { ProcessState, CrashReason, ProcessContext } from "./process.js";
import { Mailbox } from "./mailbox.js";

export class Scheduler {
  private processes: Map<ProcessId, ProcessContext> = new Map();
  private currentProcess: ProcessId | null = null;

  spawn(fn: (ctx: ProcessContext) => Promise<void>, parent?: ProcessId): ProcessId {
    const id = crypto.randomUUID();
    const ctx: ProcessContext = {
      id,
      state: "running",
      mailbox: new Mailbox(),
      parent: parent ?? null,
      links: new Set(),
      monitors: new Set(),
    };
    this.processes.set(id, ctx);

    // Run the process function, catch crashes
    const prevProcess = this.currentProcess;
    this.currentProcess = id;
    const promise = fn(ctx);
    this.currentProcess = prevProcess;

    promise.then(() => {
      // Don't overwrite if already marked as failed by linked crash
      if (ctx.state !== "failed") {
        ctx.state = "done";
        this.notifyLinked(id, { _tag: "Normal" });
      }
    }).catch((err: unknown) => {
      // Don't overwrite if already marked as failed by linked crash
      if (ctx.state !== "failed") {
        ctx.state = "failed";
        const message = err instanceof Error ? err.message : String(err);
        const stack = err instanceof Error ? err.stack : undefined;
        ctx.crashReason = { _tag: "Error", message, stack };
        this.notifyLinked(id, ctx.crashReason);
      }
    });

    return id;
  }

  send(pid: ProcessId, msg: unknown): void {
    const ctx = this.processes.get(pid);
    if (ctx) ctx.mailbox.send(msg);
  }

  async receive(pid: ProcessId): Promise<unknown> {
    const ctx = this.processes.get(pid);
    if (!ctx) throw new Error("No such process");
    ctx.state = "waiting";
    const msg = await ctx.mailbox.receive();
    ctx.state = "running";
    return msg;
  }

  self(): ProcessId {
    return this.currentProcess!;
  }

  link(a: ProcessId, b: ProcessId): void {
    const ctxA = this.processes.get(a);
    const ctxB = this.processes.get(b);
    if (ctxA && ctxB) {
      ctxA.links.add(b);
      ctxB.links.add(a);
    }
  }

  monitor(watcher: ProcessId, target: ProcessId): void {
    const ctx = this.processes.get(target);
    if (ctx) {
      ctx.monitors.add(watcher);
    }
  }

  private notifyLinked(pid: ProcessId, reason: CrashReason): void {
    const ctx = this.processes.get(pid);
    if (!ctx) return;

    // Notify linked processes
    for (const linkedPid of ctx.links) {
      const linkedCtx = this.processes.get(linkedPid);
      if (linkedCtx && linkedCtx.state !== "done" && linkedCtx.state !== "failed") {
        if (reason._tag !== "Normal") {
          linkedCtx.state = "failed";
          linkedCtx.crashReason = { _tag: "LinkedCrash", pid, reason };
          // Send crash notification to the linked process mailbox
          linkedCtx.mailbox.send({
            _type: "EXIT",
            pid,
            reason,
          });
          this.notifyLinked(linkedPid, linkedCtx.crashReason);
        }
      }
    }

    // Notify monitors
    for (const monitorPid of ctx.monitors) {
      const monitorCtx = this.processes.get(monitorPid);
      if (monitorCtx && monitorCtx.state !== "done" && monitorCtx.state !== "failed") {
        monitorCtx.mailbox.send({
          _type: "DOWN",
          pid,
          reason,
        });
      }
    }
  }

  getProcess(pid: ProcessId): ProcessContext | undefined {
    return this.processes.get(pid);
  }

  processCount(): number {
    return this.processes.size;
  }
}

// Global scheduler instance
export const scheduler = new Scheduler();

// Convenience functions that generated code calls
export function spawn(fn: (ctx: ProcessContext) => Promise<void>): ProcessId {
  return scheduler.spawn(fn);
}

export function send(pid: ProcessId, msg: unknown): void {
  scheduler.send(pid, msg);
}

export async function receive<T>(pid: ProcessId): Promise<T> {
  return scheduler.receive(pid) as Promise<T>;
}

export function self(): ProcessId {
  return scheduler.self();
}

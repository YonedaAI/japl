import type { ProcessId } from "./types.js";
import type { CrashReason, ProcessContext } from "./process.js";
import { Scheduler } from "./scheduler.js";

export type Strategy = "one_for_one" | "all_for_one" | "rest_for_one";
export type RestartPolicy = "permanent" | "transient" | "temporary";

export interface ChildSpec {
  id: string;
  start: () => ProcessId;
  restart: RestartPolicy;
}

export interface SupervisorOpts {
  strategy: Strategy;
  maxRestarts: number;
  maxSeconds: number;
  children: ChildSpec[];
}

export class Supervisor {
  private opts: SupervisorOpts;
  private children: Map<string, { spec: ChildSpec; pid: ProcessId }> = new Map();
  private childOrder: string[] = [];
  private restartLog: number[] = [];
  private supervisorPid: ProcessId | null = null;
  private scheduler: Scheduler;
  private stopped = false;

  constructor(opts: SupervisorOpts, scheduler?: Scheduler) {
    this.opts = opts;
    this.scheduler = scheduler ?? new Scheduler();
  }

  start(): ProcessId {
    this.supervisorPid = this.scheduler.spawn(async (ctx: ProcessContext) => {
      // Set supervisorPid from the context since spawn hasn't returned yet
      this.supervisorPid = ctx.id;

      // Start all children
      for (const spec of this.opts.children) {
        this.startChild(spec);
      }

      // Monitor loop: listen for DOWN messages
      while (!this.stopped) {
        const msg = await ctx.mailbox.receive() as { _type: string; pid?: ProcessId; reason?: CrashReason; childId?: string };

        if (msg._type === "STOP") {
          this.stopped = true;
          break;
        }

        if (msg._type === "DOWN" && msg.pid && msg.reason) {
          // Find which child crashed
          for (const [childId, child] of this.children) {
            if (child.pid === msg.pid) {
              this.handleChildCrash(childId, msg.reason);
              break;
            }
          }
        }
      }
    });

    return this.supervisorPid;
  }

  private startChild(spec: ChildSpec): ProcessId {
    const pid = spec.start();
    this.children.set(spec.id, { spec, pid });
    if (!this.childOrder.includes(spec.id)) {
      this.childOrder.push(spec.id);
    }
    // Monitor the child
    if (this.supervisorPid) {
      this.scheduler.monitor(this.supervisorPid, pid);
    }
    return pid;
  }

  private handleChildCrash(childId: string, reason: CrashReason): void {
    const child = this.children.get(childId);
    if (!child) return;

    if (!this.shouldRestart(child.spec, reason)) {
      this.children.delete(childId);
      return;
    }

    if (!this.checkRestartIntensity()) {
      // Too many restarts, stop the supervisor
      this.stopped = true;
      return;
    }

    switch (this.opts.strategy) {
      case "one_for_one":
        this.restartOne(childId);
        break;
      case "all_for_one":
        this.restartAll();
        break;
      case "rest_for_one":
        this.restartRest(childId);
        break;
    }
  }

  private shouldRestart(spec: ChildSpec, reason: CrashReason): boolean {
    switch (spec.restart) {
      case "permanent":
        return true;
      case "transient":
        return reason._tag !== "Normal";
      case "temporary":
        return false;
    }
  }

  private checkRestartIntensity(): boolean {
    const now = Date.now();
    this.restartLog.push(now);

    // Remove restarts outside the window
    const windowStart = now - this.opts.maxSeconds * 1000;
    this.restartLog = this.restartLog.filter(t => t >= windowStart);

    return this.restartLog.length <= this.opts.maxRestarts;
  }

  private restartOne(childId: string): void {
    const child = this.children.get(childId);
    if (!child) return;
    this.startChild(child.spec);
  }

  private restartAll(): void {
    for (const childId of this.childOrder) {
      const child = this.children.get(childId);
      if (child) {
        this.startChild(child.spec);
      }
    }
  }

  private restartRest(afterChildId: string): void {
    const idx = this.childOrder.indexOf(afterChildId);
    if (idx === -1) return;
    for (let i = idx; i < this.childOrder.length; i++) {
      const childId = this.childOrder[i];
      const child = this.children.get(childId);
      if (child) {
        this.startChild(child.spec);
      }
    }
  }

  getChildren(): Map<string, { spec: ChildSpec; pid: ProcessId }> {
    return this.children;
  }

  getScheduler(): Scheduler {
    return this.scheduler;
  }
}

export function startSupervisor(opts: SupervisorOpts, scheduler?: Scheduler): { pid: ProcessId; supervisor: Supervisor } {
  const sup = new Supervisor(opts, scheduler);
  const pid = sup.start();
  return { pid, supervisor: sup };
}

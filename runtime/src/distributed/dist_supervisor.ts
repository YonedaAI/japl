import { DistributedRuntime } from './distributed_runtime.js';
import type { DistributedPid } from './dpid.js';
import type { ProcessContext } from '../process.js';

export type DistStrategy = 'one_for_one' | 'all_for_one' | 'rest_for_one';
export type DistRestartPolicy = 'permanent' | 'transient' | 'temporary';

export interface DistChildSpec {
  id: string;
  node: string;               // which node to spawn on
  fallbackNode?: string;       // if primary node down, spawn here
  start: () => Promise<void>;  // the process function
  restart: DistRestartPolicy;
}

export interface DistSupervisorOpts {
  strategy: DistStrategy;
  maxRestarts: number;
  maxSeconds: number;
  children: DistChildSpec[];
}

export class DistSupervisor {
  private runtime: DistributedRuntime;
  private opts: DistSupervisorOpts;
  private children: Map<string, { spec: DistChildSpec; pid: DistributedPid | null }> = new Map();
  private childOrder: string[] = [];
  private restartLog: number[] = [];
  private running = false;

  constructor(runtime: DistributedRuntime, opts: DistSupervisorOpts) {
    this.runtime = runtime;
    this.opts = opts;
  }

  async start(): Promise<void> {
    this.running = true;
    // Start all children in order
    for (const spec of this.opts.children) {
      await this.startChild(spec);
    }
  }

  private async startChild(spec: DistChildSpec): Promise<void> {
    const targetNode = spec.node;
    let pid: DistributedPid;

    if (targetNode === this.runtime.selfNode()) {
      // Local spawn
      pid = this.runtime.spawn(async (_ctx: ProcessContext) => {
        await spec.start();
      });
    } else {
      // Remote spawn — try primary node, fallback if down
      try {
        pid = await this.runtime.spawnRemote(targetNode, '', '', []);
      } catch {
        if (spec.fallbackNode) {
          console.log(`[supervisor] ${spec.id}: ${targetNode} unavailable, using fallback ${spec.fallbackNode}`);
          if (spec.fallbackNode === this.runtime.selfNode()) {
            pid = this.runtime.spawn(async (_ctx: ProcessContext) => {
              await spec.start();
            });
          } else {
            pid = await this.runtime.spawnRemote(spec.fallbackNode, '', '', []);
          }
        } else {
          throw new Error(`Cannot start ${spec.id}: node ${targetNode} unavailable`);
        }
      }
    }

    this.children.set(spec.id, { spec, pid });
    if (!this.childOrder.includes(spec.id)) {
      this.childOrder.push(spec.id);
    }

    // Monitor remote children
    if (pid.node !== this.runtime.selfNode()) {
      // Use the local supervisor's own process as watcher
      this.runtime.monitorRemote(pid.local, pid);
    }
  }

  /** Called when a child process exits. */
  async handleChildExit(childId: string, reason: string): Promise<void> {
    const child = this.children.get(childId);
    if (!child) return;

    const shouldRestart = this.shouldRestart(child.spec, reason);
    if (!shouldRestart) {
      child.pid = null;
      return;
    }

    // Check restart intensity
    if (!this.checkRestartIntensity()) {
      console.error(`[supervisor] Max restart intensity reached. Shutting down.`);
      await this.shutdown();
      return;
    }

    // Apply restart strategy
    switch (this.opts.strategy) {
      case 'one_for_one':
        await this.startChild(child.spec);
        break;
      case 'all_for_one':
        await this.restartAll();
        break;
      case 'rest_for_one':
        await this.restartRest(childId);
        break;
    }
  }

  /** Called when an entire node goes down. */
  async handleNodeDown(nodeName: string): Promise<void> {
    for (const [id, child] of this.children) {
      if (child.pid?.node === nodeName) {
        console.log(`[supervisor] ${id}: node ${nodeName} down, restarting...`);
        child.pid = null;
        await this.handleChildExit(id, 'node_down');
      }
    }
  }

  private shouldRestart(spec: DistChildSpec, reason: string): boolean {
    switch (spec.restart) {
      case 'permanent': return true;
      case 'transient': return reason !== 'normal';
      case 'temporary': return false;
    }
  }

  private checkRestartIntensity(): boolean {
    const now = Date.now();
    this.restartLog.push(now);
    // Remove old entries outside the window
    const windowStart = now - (this.opts.maxSeconds * 1000);
    this.restartLog = this.restartLog.filter(t => t >= windowStart);
    return this.restartLog.length <= this.opts.maxRestarts;
  }

  private async restartAll(): Promise<void> {
    for (const id of this.childOrder) {
      const child = this.children.get(id);
      if (child) {
        child.pid = null;
        await this.startChild(child.spec);
      }
    }
  }

  private async restartRest(afterChildId: string): Promise<void> {
    const idx = this.childOrder.indexOf(afterChildId);
    if (idx === -1) return;
    for (let i = idx; i < this.childOrder.length; i++) {
      const id = this.childOrder[i];
      const child = this.children.get(id);
      if (child) {
        child.pid = null;
        await this.startChild(child.spec);
      }
    }
  }

  async shutdown(): Promise<void> {
    this.running = false;
    this.children.clear();
    this.childOrder = [];
  }

  isRunning(): boolean {
    return this.running;
  }

  getChildren(): Map<string, { spec: DistChildSpec; pid: DistributedPid | null }> {
    return new Map(this.children);
  }
}

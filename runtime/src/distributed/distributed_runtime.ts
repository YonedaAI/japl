import { Scheduler } from '../scheduler.js';
import { ConnectionManager } from '../node/connection.js';
import type { ConnectionCallbacks } from '../node/connection.js';
import { HealthMonitor } from '../node/health.js';
import { NodeRegistry } from '../node/registry.js';
import type { NodeConfig } from '../node/node.js';
import { MsgType } from '../wire/protocol.js';
import type { WireMessage } from '../wire/protocol.js';
import { DistributedRouter } from './router.js';
import { RemoteSpawner } from './remote_spawn.js';
import { RemoteMonitor } from './remote_monitor.js';
import type { DistributedPid } from './dpid.js';
import { serialize } from '../wire/serialize.js';
import {
  decodeExitPayload,
  encodeSpawnResponsePayload,
  decodeSpawnRequestPayload,
  encodeLookupPayload,
  decodeLookupResponsePayload,
  encodeLookupResponsePayload,
} from '../wire/frame.js';
import type { ProcessContext } from '../process.js';

export class DistributedRuntime {
  public scheduler: Scheduler;
  public router: DistributedRouter;
  public spawner: RemoteSpawner;
  public monitor: RemoteMonitor;
  public health: HealthMonitor;
  public registry: NodeRegistry;
  public connections: ConnectionManager;
  public config: NodeConfig;

  private pendingLookups: Map<string, {
    resolve: (pid: DistributedPid | null) => void;
    timer: ReturnType<typeof setTimeout>;
  }> = new Map();

  constructor(config: NodeConfig) {
    this.config = config;
    this.scheduler = new Scheduler();
    this.registry = new NodeRegistry();

    const callbacks: ConnectionCallbacks = {
      onMessage: (from, msg) => this.handleMessage(from, msg),
      onNodeUp: (name) => this.handleNodeUp(name),
      onNodeDown: (name) => this.handleNodeDown(name),
    };

    this.connections = new ConnectionManager(config, callbacks);
    this.router = new DistributedRouter(config.name, this.scheduler, this.connections);
    this.spawner = new RemoteSpawner(config.name, this.connections);
    this.monitor = new RemoteMonitor();
    this.health = new HealthMonitor(
      {},
      (node) => this.sendPing(node),
      (node) => this.handleNodeDown(node),
    );
  }

  /** Start the runtime: listen and connect to peers. */
  async start(): Promise<void> {
    await this.connections.listen();
    await this.connections.connectToPeers();
  }

  /** Shut down the runtime. */
  async shutdown(): Promise<void> {
    this.health.shutdown();
    this.spawner.cancelAll();
    // Cancel pending lookups
    for (const [id, pending] of this.pendingLookups) {
      clearTimeout(pending.timer);
      pending.resolve(null);
    }
    this.pendingLookups.clear();
    await this.connections.shutdown();
  }

  /** Spawn a local process, returning a DistributedPid. */
  spawn(fn: (ctx: ProcessContext) => Promise<void>): DistributedPid {
    const localId = this.scheduler.spawn(fn);
    return { node: this.config.name, local: localId };
  }

  /** Spawn a process on a remote node. */
  async spawnRemote(
    targetNode: string,
    module: string,
    fn: string,
    args: unknown[],
  ): Promise<DistributedPid> {
    return this.spawner.spawnRemote(targetNode, module, fn, serialize(args));
  }

  /** Send a message to any process (local or remote). */
  send(pid: DistributedPid, msg: unknown): void {
    this.router.send(pid, msg);
  }

  /** Register a named process locally. */
  register(name: string, pid: DistributedPid): void {
    this.router.register(name, pid);
  }

  /** Look up a named process locally. */
  async lookup(name: string): Promise<DistributedPid | null> {
    return this.router.lookup(name);
  }

  /** Look up a named process on a specific remote node. */
  async lookupRemote(nodeName: string, name: string, timeoutMs: number = 3000): Promise<DistributedPid | null> {
    const requestId = crypto.randomUUID();
    const payload = encodeLookupPayload({ requestId, name });

    return new Promise<DistributedPid | null>((resolve) => {
      const timer = setTimeout(() => {
        this.pendingLookups.delete(requestId);
        resolve(null);
      }, timeoutMs);

      this.pendingLookups.set(requestId, { resolve, timer });

      const sent = this.connections.send(nodeName, {
        type: MsgType.LOOKUP,
        fromNode: this.config.name,
        toNode: nodeName,
        payload,
      });

      if (!sent) {
        clearTimeout(timer);
        this.pendingLookups.delete(requestId);
        resolve(null);
      }
    });
  }

  /** Monitor a remote process. */
  monitorRemote(watcherLocalPid: string, targetPid: DistributedPid): void {
    this.monitor.monitor(watcherLocalPid, targetPid, this.connections, this.config.name);
  }

  /** Get this node's name. */
  selfNode(): string {
    return this.config.name;
  }

  // ---------------------------------------------------------------------------
  // Internal message dispatch
  // ---------------------------------------------------------------------------

  private handleMessage(from: string, msg: WireMessage): void {
    switch (msg.type) {
      case MsgType.SPAWN_RESPONSE:
        this.spawner.handleSpawnResponse(from, msg.payload);
        break;

      case MsgType.SPAWN_REQUEST:
        this.handleSpawnRequest(from, msg.payload);
        break;

      case MsgType.EXIT:
        this.monitor.handleRemoteExit(from, decodeExitPayload(msg.payload), this.scheduler);
        break;

      case MsgType.LOOKUP_RESPONSE: {
        const resp = decodeLookupResponsePayload(msg.payload);
        const pending = this.pendingLookups.get(resp.requestId);
        if (pending) {
          clearTimeout(pending.timer);
          this.pendingLookups.delete(resp.requestId);
          if (resp.pid !== null) {
            const colonIdx = resp.pid.indexOf(':');
            if (colonIdx >= 0) {
              pending.resolve({ node: resp.pid.slice(0, colonIdx), local: resp.pid.slice(colonIdx + 1) });
            } else {
              pending.resolve({ node: from, local: resp.pid });
            }
          } else {
            pending.resolve(null);
          }
        }
        // Also let the router handle it (for router-level lookups)
        this.router.handleIncoming(from, msg);
        break;
      }

      default:
        // Delegate to router for SEND, LOOKUP, etc.
        this.router.handleIncoming(from, msg);
        break;
    }
  }

  private handleSpawnRequest(from: string, payload: Uint8Array): void {
    const req = decodeSpawnRequestPayload(payload);
    // Spawn a local process that represents the remote request
    const localId = this.scheduler.spawn(async (_ctx) => {
      // In a full implementation, this would load the module and call the function.
      // For now, the process is spawned and its pid returned.
    });

    const responsePayload = encodeSpawnResponsePayload({
      requestId: req.requestId,
      pid: localId,
    });

    this.connections.send(from, {
      type: MsgType.SPAWN_RESPONSE,
      fromNode: this.config.name,
      toNode: from,
      payload: responsePayload,
    });
  }

  private handleNodeUp(name: string): void {
    this.health.startMonitoring(name);
    this.registry.addNode(name, {
      name,
      host: '',
      port: 0,
      connectedAt: Date.now(),
      status: 'up',
    });
  }

  private handleNodeDown(name: string): void {
    this.health.stopMonitoring(name);
    this.registry.removeNode(name);
    this.monitor.handleNodeDown(name, this.scheduler);
  }

  private sendPing(node: string): void {
    this.connections.send(node, {
      type: MsgType.PING,
      fromNode: this.config.name,
      toNode: node,
      payload: new Uint8Array(0),
    });
  }
}

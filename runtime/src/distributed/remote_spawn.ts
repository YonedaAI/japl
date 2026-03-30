import type { ConnectionManager } from '../node/connection.js';
import { MsgType } from '../wire/protocol.js';
import type { WireMessage } from '../wire/protocol.js';
import { encodeSpawnRequestPayload, decodeSpawnResponsePayload } from '../wire/frame.js';
import type { DistributedPid } from './dpid.js';

export class RemoteSpawner {
  private pendingSpawns: Map<string, { resolve: (pid: DistributedPid) => void; reject: (err: Error) => void; timer: ReturnType<typeof setTimeout> }> = new Map();
  private selfNode: string;
  private connections: ConnectionManager;
  private timeoutMs: number;

  constructor(selfNode: string, connections: ConnectionManager, timeoutMs: number = 10000) {
    this.selfNode = selfNode;
    this.connections = connections;
    this.timeoutMs = timeoutMs;
  }

  /** Spawn a process on a remote node. */
  async spawnRemote(targetNode: string, moduleName: string, fnName: string, args: Uint8Array): Promise<DistributedPid> {
    const requestId = crypto.randomUUID();

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        if (this.pendingSpawns.has(requestId)) {
          this.pendingSpawns.delete(requestId);
          reject(new Error(`Spawn on ${targetNode} timed out`));
        }
      }, this.timeoutMs);

      this.pendingSpawns.set(requestId, { resolve, reject, timer });

      const payload = encodeSpawnRequestPayload({
        requestId,
        module: moduleName,
        fn: fnName,
        args,
      });

      const sent = this.connections.send(targetNode, {
        type: MsgType.SPAWN_REQUEST,
        fromNode: this.selfNode,
        toNode: targetNode,
        payload,
      });

      if (!sent) {
        clearTimeout(timer);
        this.pendingSpawns.delete(requestId);
        reject(new Error(`Cannot reach node ${targetNode}`));
      }
    });
  }

  /** Handle SPAWN_RESPONSE from a remote node. */
  handleSpawnResponse(fromNode: string, payload: Uint8Array): void {
    const resp = decodeSpawnResponsePayload(payload);
    const pending = this.pendingSpawns.get(resp.requestId);
    if (pending) {
      clearTimeout(pending.timer);
      this.pendingSpawns.delete(resp.requestId);
      pending.resolve({ node: fromNode, local: resp.pid });
    }
  }

  /** Number of in-flight spawn requests (for testing). */
  get pendingCount(): number {
    return this.pendingSpawns.size;
  }

  /** Cancel all pending spawns (used during shutdown). */
  cancelAll(): void {
    for (const [id, pending] of this.pendingSpawns) {
      clearTimeout(pending.timer);
      pending.reject(new Error('Spawner shutting down'));
    }
    this.pendingSpawns.clear();
  }
}

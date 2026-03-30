// Distributed message router — sits between application code and scheduler/network.
// When code calls send(pid, msg), the router decides: local or remote delivery.

import type { ConnectionManager } from "../node/connection.js";
import type { Scheduler } from "../scheduler.js";
import { serialize } from "../wire/serialize.js";
import { deserialize } from "../wire/deserialize.js";
import type { DistributedPid } from "./dpid.js";
import { makePid, pidToString } from "./dpid.js";
import { MsgType } from "../wire/protocol.js";
import type { WireMessage, LookupResponsePayload } from "../wire/protocol.js";
import {
  encodeSendPayload,
  decodeSendPayload,
  encodeLookupPayload,
  decodeLookupPayload,
  encodeLookupResponsePayload,
  decodeLookupResponsePayload,
} from "../wire/frame.js";

export class DistributedRouter {
  private selfNode: string;
  private scheduler: Scheduler;
  private connections: ConnectionManager;
  private nameRegistry: Map<string, DistributedPid> = new Map();
  private pendingLookups: Map<string, {
    resolve: (pid: DistributedPid | null) => void;
    remaining: number;
    timer: ReturnType<typeof setTimeout>;
  }> = new Map();

  constructor(selfNode: string, scheduler: Scheduler, connections: ConnectionManager) {
    this.selfNode = selfNode;
    this.scheduler = scheduler;
    this.connections = connections;
  }

  /** Send a message — routes automatically based on target node. */
  send(to: DistributedPid, msg: unknown): boolean {
    if (to.node === this.selfNode) {
      // Local delivery — use scheduler directly
      this.scheduler.send(to.local, msg);
      return true;
    } else {
      // Remote delivery — serialize and send over TCP
      const payload = encodeSendPayload({
        toPid: to.local,
        fromPid: "",
        data: serialize(msg),
      });
      return this.connections.send(to.node, {
        type: MsgType.SEND,
        fromNode: this.selfNode,
        toNode: to.node,
        payload,
      });
    }
  }

  /** Handle incoming message from remote node. */
  handleIncoming(from: string, msg: WireMessage): void {
    switch (msg.type) {
      case MsgType.SEND: {
        const payload = decodeSendPayload(msg.payload);
        const result = deserialize(payload.data);
        this.scheduler.send(payload.toPid, result.value);
        break;
      }

      case MsgType.LOOKUP: {
        const lookup = decodeLookupPayload(msg.payload);
        const found = this.nameRegistry.get(lookup.name);
        const responsePid = found ? pidToString(found) : null;
        const responsePayload = encodeLookupResponsePayload({
          requestId: lookup.requestId,
          pid: responsePid,
        });
        this.connections.send(from, {
          type: MsgType.LOOKUP_RESPONSE,
          fromNode: this.selfNode,
          toNode: from,
          payload: responsePayload,
        });
        break;
      }

      case MsgType.LOOKUP_RESPONSE: {
        const resp = decodeLookupResponsePayload(msg.payload);
        const pending = this.pendingLookups.get(resp.requestId);
        if (pending) {
          if (resp.pid !== null) {
            // Found — resolve immediately
            clearTimeout(pending.timer);
            this.pendingLookups.delete(resp.requestId);
            // Parse the pid string back: it's "node:local" format
            const colonIdx = resp.pid.indexOf(":");
            if (colonIdx >= 0) {
              pending.resolve(makePid(resp.pid.slice(0, colonIdx), resp.pid.slice(colonIdx + 1)));
            } else {
              // Treat as local pid on the responding node
              pending.resolve(makePid(from, resp.pid));
            }
          } else {
            pending.remaining--;
            if (pending.remaining <= 0) {
              clearTimeout(pending.timer);
              this.pendingLookups.delete(resp.requestId);
              pending.resolve(null);
            }
          }
        }
        break;
      }

      default:
        // Other message types can be extended later
        break;
    }
  }

  /** Register a named process. */
  register(name: string, pid: DistributedPid): void {
    this.nameRegistry.set(name, pid);
  }

  /** Unregister a named process. */
  unregister(name: string): void {
    this.nameRegistry.delete(name);
  }

  /** Look up a named process — checks local registry, then asks connected nodes. */
  async lookup(name: string, timeoutMs: number = 3000): Promise<DistributedPid | null> {
    // Check local registry first
    const local = this.nameRegistry.get(name);
    if (local) return local;

    // Ask connected nodes
    const connectedNodes = this.connections.getConnectedNodes();
    if (connectedNodes.length === 0) return null;

    const requestId = crypto.randomUUID();
    const payload = encodeLookupPayload({ requestId, name });

    return new Promise<DistributedPid | null>((resolve) => {
      const timer = setTimeout(() => {
        this.pendingLookups.delete(requestId);
        resolve(null);
      }, timeoutMs);

      this.pendingLookups.set(requestId, {
        resolve,
        remaining: connectedNodes.length,
        timer,
      });

      for (const node of connectedNodes) {
        this.connections.send(node, {
          type: MsgType.LOOKUP,
          fromNode: this.selfNode,
          toNode: node,
          payload,
        });
      }
    });
  }

  /** Get self node name. */
  getSelfNode(): string {
    return this.selfNode;
  }

  /** Get the local name registry (for testing / inspection). */
  getNameRegistry(): ReadonlyMap<string, DistributedPid> {
    return this.nameRegistry;
  }
}

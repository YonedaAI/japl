import type { ConnectionManager } from '../node/connection.js';
import type { Scheduler } from '../scheduler.js';
import { MsgType } from '../wire/protocol.js';
import type { ExitPayload } from '../wire/protocol.js';
import { encodeMonitorPayload } from '../wire/frame.js';
import type { DistributedPid } from './dpid.js';

export class RemoteMonitor {
  /** remote pid key ("node:local") -> set of local watcher pids. */
  private monitors: Map<string, Set<string>> = new Map();

  /** When a local process monitors a remote process. */
  monitor(
    watcherLocalPid: string,
    targetPid: DistributedPid,
    connections: ConnectionManager,
    selfNode: string,
  ): void {
    const key = `${targetPid.node}:${targetPid.local}`;
    if (!this.monitors.has(key)) this.monitors.set(key, new Set());
    this.monitors.get(key)!.add(watcherLocalPid);

    // Send MONITOR message to the remote node
    connections.send(targetPid.node, {
      type: MsgType.MONITOR,
      fromNode: selfNode,
      toNode: targetPid.node,
      payload: encodeMonitorPayload({
        monitorPid: watcherLocalPid,
        targetPid: targetPid.local,
      }),
    });
  }

  /** When a remote process exits, notify all local watchers. */
  handleRemoteExit(
    fromNode: string,
    exitPayload: ExitPayload,
    scheduler: Scheduler,
  ): void {
    const key = `${fromNode}:${exitPayload.pid}`;
    const watchers = this.monitors.get(key);
    if (watchers) {
      for (const watcher of watchers) {
        scheduler.send(watcher, {
          _tag: 'ProcessDown',
          _0: { node: fromNode, local: exitPayload.pid },
          _1: exitPayload.reason,
        });
      }
      this.monitors.delete(key);
    }
  }

  /** When a node goes down, notify watchers of ALL processes on that node. */
  handleNodeDown(nodeName: string, scheduler: Scheduler): void {
    const keysToDelete: string[] = [];
    for (const [key, watchers] of this.monitors) {
      if (key.startsWith(nodeName + ':')) {
        for (const watcher of watchers) {
          scheduler.send(watcher, { _tag: 'NodeDown', _0: nodeName });
        }
        keysToDelete.push(key);
      }
    }
    for (const key of keysToDelete) {
      this.monitors.delete(key);
    }
  }

  /** Number of monitored remote processes (for testing). */
  get monitorCount(): number {
    return this.monitors.size;
  }

  /** Get watchers for a given remote pid key (for testing). */
  getWatchers(node: string, localPid: string): ReadonlySet<string> | undefined {
    return this.monitors.get(`${node}:${localPid}`);
  }
}

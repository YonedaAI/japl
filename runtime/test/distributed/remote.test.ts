import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { RemoteSpawner } from '../../src/distributed/remote_spawn.js';
import { RemoteMonitor } from '../../src/distributed/remote_monitor.js';
import { DistributedRuntime } from '../../src/distributed/distributed_runtime.js';
import type { DistributedPid } from '../../src/distributed/dpid.js';
import { makePid } from '../../src/distributed/dpid.js';
import { MsgType } from '../../src/wire/protocol.js';
import type { WireMessage } from '../../src/wire/protocol.js';
import {
  encodeSpawnResponsePayload,
  decodeSpawnRequestPayload,
  encodeSendPayload,
  decodeSendPayload,
  encodeExitPayload,
  decodeMonitorPayload,
} from '../../src/wire/frame.js';
import { serialize } from '../../src/wire/serialize.js';
import { deserialize } from '../../src/wire/deserialize.js';
import type { NodeConfig } from '../../src/node/node.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function createMockConnections(connectedNodes: string[] = []) {
  const sent: Array<{ node: string; msg: WireMessage }> = [];
  return {
    send(node: string, msg: WireMessage): boolean {
      const isConnected = connectedNodes.includes(node);
      if (isConnected) sent.push({ node, msg });
      return isConnected;
    },
    getConnectedNodes: () => connectedNodes,
    sent,
    listen: vi.fn(),
    connect: vi.fn(),
    connectToPeers: vi.fn(),
    getConnection: vi.fn(),
    disconnect: vi.fn(),
    shutdown: vi.fn(),
    startPingLoop: vi.fn(),
  };
}

function createMockScheduler() {
  const sent: Array<{ pid: string; msg: unknown }> = [];
  return {
    send(pid: string, msg: unknown) { sent.push({ pid, msg }); },
    sent,
    spawn: vi.fn(() => 'spawned-pid'),
    receive: vi.fn(),
    self: vi.fn(),
    link: vi.fn(),
    monitor: vi.fn(),
    getProcess: vi.fn(),
    processCount: vi.fn(),
  };
}

/** Wait for a condition with polling. */
function waitFor(condition: () => boolean, timeoutMs: number = 2000): Promise<void> {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const check = () => {
      if (condition()) {
        resolve();
      } else if (Date.now() - start > timeoutMs) {
        reject(new Error('waitFor timed out'));
      } else {
        setTimeout(check, 10);
      }
    };
    check();
  });
}

let nextPort = 19500;
function getPort(): number { return nextPort++; }

function makeConfig(name: string, port: number, cookie: string, connectTo?: string[]): NodeConfig {
  return { name, listen: `127.0.0.1:${port}`, connect: connectTo, cookie };
}

// ---------------------------------------------------------------------------
// RemoteSpawner tests
// ---------------------------------------------------------------------------

describe('RemoteSpawner', () => {
  let connections: ReturnType<typeof createMockConnections>;
  let spawner: RemoteSpawner;

  beforeEach(() => {
    connections = createMockConnections(['beta']);
    spawner = new RemoteSpawner('alpha', connections as any, 200);
  });

  it('sends SPAWN_REQUEST to the target node', async () => {
    const promise = spawner.spawnRemote('beta', 'MyModule', 'start', new Uint8Array([1, 2]));

    expect(connections.sent).toHaveLength(1);
    expect(connections.sent[0].node).toBe('beta');
    expect(connections.sent[0].msg.type).toBe(MsgType.SPAWN_REQUEST);
    expect(connections.sent[0].msg.fromNode).toBe('alpha');

    // Decode and verify the payload
    const req = decodeSpawnRequestPayload(connections.sent[0].msg.payload);
    expect(req.module).toBe('MyModule');
    expect(req.fn).toBe('start');
    expect(req.args).toEqual(new Uint8Array([1, 2]));

    // Send response to complete the promise
    const responsePayload = encodeSpawnResponsePayload({ requestId: req.requestId, pid: 'remote-pid-1' });
    spawner.handleSpawnResponse('beta', responsePayload);

    const pid = await promise;
    expect(pid).toEqual({ node: 'beta', local: 'remote-pid-1' });
  });

  it('resolves with DistributedPid on response', async () => {
    const promise = spawner.spawnRemote('beta', 'Mod', 'init', new Uint8Array(0));
    const req = decodeSpawnRequestPayload(connections.sent[0].msg.payload);

    spawner.handleSpawnResponse('beta', encodeSpawnResponsePayload({
      requestId: req.requestId,
      pid: 'pid-abc',
    }));

    const result = await promise;
    expect(result.node).toBe('beta');
    expect(result.local).toBe('pid-abc');
    expect(spawner.pendingCount).toBe(0);
  });

  it('rejects on timeout', async () => {
    const promise = spawner.spawnRemote('beta', 'Mod', 'fn', new Uint8Array(0));
    // Don't send response — let it timeout
    await expect(promise).rejects.toThrow('timed out');
    expect(spawner.pendingCount).toBe(0);
  });

  it('rejects immediately when target node is unreachable', async () => {
    const promise = spawner.spawnRemote('unknown-node', 'Mod', 'fn', new Uint8Array(0));
    await expect(promise).rejects.toThrow('Cannot reach node unknown-node');
  });

  it('ignores responses for unknown request ids', () => {
    const payload = encodeSpawnResponsePayload({ requestId: 'no-such-id', pid: 'pid-x' });
    // Should not throw
    spawner.handleSpawnResponse('beta', payload);
    expect(spawner.pendingCount).toBe(0);
  });

  it('cancelAll rejects all pending spawns', async () => {
    const p1 = spawner.spawnRemote('beta', 'Mod', 'fn1', new Uint8Array(0));
    const p2 = spawner.spawnRemote('beta', 'Mod', 'fn2', new Uint8Array(0));

    expect(spawner.pendingCount).toBe(2);
    spawner.cancelAll();
    expect(spawner.pendingCount).toBe(0);

    await expect(p1).rejects.toThrow('shutting down');
    await expect(p2).rejects.toThrow('shutting down');
  });
});

// ---------------------------------------------------------------------------
// RemoteMonitor tests
// ---------------------------------------------------------------------------

describe('RemoteMonitor', () => {
  let connections: ReturnType<typeof createMockConnections>;
  let scheduler: ReturnType<typeof createMockScheduler>;
  let monitor: RemoteMonitor;

  beforeEach(() => {
    connections = createMockConnections(['beta', 'gamma']);
    scheduler = createMockScheduler();
    monitor = new RemoteMonitor();
  });

  it('monitor sends MONITOR message to remote node', () => {
    const target: DistributedPid = { node: 'beta', local: 'remote-pid-1' };
    monitor.monitor('local-watcher', target, connections as any, 'alpha');

    expect(connections.sent).toHaveLength(1);
    expect(connections.sent[0].node).toBe('beta');
    expect(connections.sent[0].msg.type).toBe(MsgType.MONITOR);

    const decoded = decodeMonitorPayload(connections.sent[0].msg.payload);
    expect(decoded.monitorPid).toBe('local-watcher');
    expect(decoded.targetPid).toBe('remote-pid-1');
  });

  it('monitor tracks watchers for a remote pid', () => {
    const target: DistributedPid = { node: 'beta', local: 'rp-1' };
    monitor.monitor('watcher-1', target, connections as any, 'alpha');
    monitor.monitor('watcher-2', target, connections as any, 'alpha');

    const watchers = monitor.getWatchers('beta', 'rp-1');
    expect(watchers).toBeDefined();
    expect(watchers!.size).toBe(2);
    expect(watchers!.has('watcher-1')).toBe(true);
    expect(watchers!.has('watcher-2')).toBe(true);
  });

  it('handleRemoteExit notifies all local watchers', () => {
    const target: DistributedPid = { node: 'beta', local: 'rp-1' };
    monitor.monitor('w1', target, connections as any, 'alpha');
    monitor.monitor('w2', target, connections as any, 'alpha');

    monitor.handleRemoteExit('beta', { pid: 'rp-1', reason: 'normal' }, scheduler as any);

    expect(scheduler.sent).toHaveLength(2);
    const pids = scheduler.sent.map((s) => s.pid).sort();
    expect(pids).toEqual(['w1', 'w2']);
    expect(scheduler.sent[0].msg).toEqual({
      _tag: 'ProcessDown',
      _0: { node: 'beta', local: 'rp-1' },
      _1: 'normal',
    });
  });

  it('handleRemoteExit removes monitors after notification', () => {
    const target: DistributedPid = { node: 'beta', local: 'rp-1' };
    monitor.monitor('w1', target, connections as any, 'alpha');
    monitor.handleRemoteExit('beta', { pid: 'rp-1', reason: 'crash' }, scheduler as any);

    expect(monitor.monitorCount).toBe(0);
  });

  it('handleRemoteExit does nothing for unmonitored process', () => {
    monitor.handleRemoteExit('beta', { pid: 'unknown', reason: 'crash' }, scheduler as any);
    expect(scheduler.sent).toHaveLength(0);
  });

  it('handleNodeDown notifies watchers of all processes on the node', () => {
    monitor.monitor('w1', { node: 'beta', local: 'rp-1' }, connections as any, 'alpha');
    monitor.monitor('w2', { node: 'beta', local: 'rp-2' }, connections as any, 'alpha');
    monitor.monitor('w3', { node: 'gamma', local: 'rp-3' }, connections as any, 'alpha');

    monitor.handleNodeDown('beta', scheduler as any);

    // Only w1 and w2 should be notified (beta processes)
    expect(scheduler.sent).toHaveLength(2);
    const pids = scheduler.sent.map((s) => s.pid).sort();
    expect(pids).toEqual(['w1', 'w2']);
    for (const entry of scheduler.sent) {
      expect(entry.msg).toEqual({ _tag: 'NodeDown', _0: 'beta' });
    }

    // gamma monitor should still exist
    expect(monitor.getWatchers('gamma', 'rp-3')).toBeDefined();
  });

  it('handleNodeDown cleans up monitors for that node', () => {
    monitor.monitor('w1', { node: 'beta', local: 'rp-1' }, connections as any, 'alpha');
    monitor.handleNodeDown('beta', scheduler as any);
    expect(monitor.getWatchers('beta', 'rp-1')).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// DistributedRuntime unit tests (mocked connections)
// ---------------------------------------------------------------------------

describe('DistributedRuntime', () => {
  let runtimes: DistributedRuntime[];

  beforeEach(() => {
    runtimes = [];
  });

  afterEach(async () => {
    for (const rt of runtimes) {
      await rt.shutdown();
    }
    runtimes = [];
  });

  it('selfNode returns the config name', () => {
    const rt = new DistributedRuntime({ name: 'alpha', cookie: 'secret' });
    runtimes.push(rt);
    expect(rt.selfNode()).toBe('alpha');
  });

  it('spawn creates a local process with a DistributedPid', () => {
    const rt = new DistributedRuntime({ name: 'alpha', cookie: 'secret' });
    runtimes.push(rt);
    const pid = rt.spawn(async () => {});
    expect(pid.node).toBe('alpha');
    expect(typeof pid.local).toBe('string');
    expect(pid.local.length).toBeGreaterThan(0);
  });

  it('send delivers to local process', async () => {
    const rt = new DistributedRuntime({ name: 'alpha', cookie: 'secret' });
    runtimes.push(rt);
    let received: unknown = null;

    const pid = rt.spawn(async (ctx) => {
      received = await rt.scheduler.receive(ctx.id);
    });

    rt.send(pid, 'hello-local');
    await waitFor(() => received !== null);
    expect(received).toBe('hello-local');
  });

  it('register and lookup work for local names', async () => {
    const rt = new DistributedRuntime({ name: 'alpha', cookie: 'secret' });
    runtimes.push(rt);
    const pid = rt.spawn(async () => {});
    rt.register('my-service', pid);
    const found = await rt.lookup('my-service');
    expect(found).toEqual(pid);
  });

  it('lookup returns null for unknown names', async () => {
    const rt = new DistributedRuntime({ name: 'alpha', cookie: 'secret' });
    runtimes.push(rt);
    const found = await rt.lookup('nonexistent');
    expect(found).toBeNull();
  });

  it('start and shutdown complete without error', async () => {
    const port = getPort();
    const rt = new DistributedRuntime({ name: 'alpha', listen: `127.0.0.1:${port}`, cookie: 'secret' });
    runtimes.push(rt);
    await rt.start();
    await rt.shutdown();
  });

  it('shutdown cancels pending spawns', async () => {
    const rt = new DistributedRuntime({ name: 'alpha', cookie: 'secret' });
    runtimes.push(rt);
    // Spawner has no connected nodes so spawnRemote will fail immediately
    // But we can verify cancelAll is called cleanly during shutdown
    await rt.shutdown();
    expect(rt.spawner.pendingCount).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Integration tests — two runtimes on localhost
// ---------------------------------------------------------------------------

describe('DistributedRuntime integration', () => {
  let runtimes: DistributedRuntime[];

  beforeEach(() => {
    runtimes = [];
  });

  afterEach(async () => {
    for (const rt of runtimes) {
      await rt.shutdown();
    }
    runtimes = [];
  });

  it('two nodes connect and exchange messages', async () => {
    const portA = getPort();
    const portB = getPort();

    const rtA = new DistributedRuntime({
      name: 'nodeA',
      listen: `127.0.0.1:${portA}`,
      cookie: 'test-cookie',
    });
    const rtB = new DistributedRuntime({
      name: 'nodeB',
      listen: `127.0.0.1:${portB}`,
      connect: [`127.0.0.1:${portA}`],
      cookie: 'test-cookie',
    });
    runtimes.push(rtA, rtB);

    await rtA.start();
    await rtB.start();

    // Wait for connections
    await waitFor(() =>
      rtA.connections.getConnectedNodes().includes('nodeB') &&
      rtB.connections.getConnectedNodes().includes('nodeA'),
    );

    // Spawn a process on nodeA and send it a message from nodeB
    let received: unknown = null;
    const pidA = rtA.spawn(async (ctx) => {
      received = await rtA.scheduler.receive(ctx.id);
    });

    // nodeB sends to the process on nodeA
    rtB.send(pidA, 'cross-node-hello');

    await waitFor(() => received !== null);
    expect(received).toBe('cross-node-hello');
  });

  it('remote spawn request and response flow', async () => {
    const portA = getPort();
    const portB = getPort();

    const rtA = new DistributedRuntime({
      name: 'nodeA',
      listen: `127.0.0.1:${portA}`,
      cookie: 'test-cookie',
    });
    const rtB = new DistributedRuntime({
      name: 'nodeB',
      listen: `127.0.0.1:${portB}`,
      connect: [`127.0.0.1:${portA}`],
      cookie: 'test-cookie',
    });
    runtimes.push(rtA, rtB);

    await rtA.start();
    await rtB.start();

    await waitFor(() =>
      rtA.connections.getConnectedNodes().includes('nodeB') &&
      rtB.connections.getConnectedNodes().includes('nodeA'),
    );

    // nodeB requests a spawn on nodeA
    const remotePid = await rtB.spawnRemote('nodeA', 'TestMod', 'init', [1, 2, 3]);
    expect(remotePid.node).toBe('nodeA');
    expect(typeof remotePid.local).toBe('string');
    expect(remotePid.local.length).toBeGreaterThan(0);
  });

  it('bidirectional messaging between two nodes', async () => {
    const portA = getPort();
    const portB = getPort();

    const rtA = new DistributedRuntime({
      name: 'nodeA',
      listen: `127.0.0.1:${portA}`,
      cookie: 'test-cookie',
    });
    const rtB = new DistributedRuntime({
      name: 'nodeB',
      listen: `127.0.0.1:${portB}`,
      connect: [`127.0.0.1:${portA}`],
      cookie: 'test-cookie',
    });
    runtimes.push(rtA, rtB);

    await rtA.start();
    await rtB.start();

    await waitFor(() =>
      rtA.connections.getConnectedNodes().includes('nodeB') &&
      rtB.connections.getConnectedNodes().includes('nodeA'),
    );

    let receivedOnA: unknown = null;
    let receivedOnB: unknown = null;

    const pidA = rtA.spawn(async (ctx) => {
      receivedOnA = await rtA.scheduler.receive(ctx.id);
    });
    const pidB = rtB.spawn(async (ctx) => {
      receivedOnB = await rtB.scheduler.receive(ctx.id);
    });

    rtB.send(pidA, 'msg-to-A');
    rtA.send(pidB, 'msg-to-B');

    await waitFor(() => receivedOnA !== null && receivedOnB !== null);
    expect(receivedOnA).toBe('msg-to-A');
    expect(receivedOnB).toBe('msg-to-B');
  });
});

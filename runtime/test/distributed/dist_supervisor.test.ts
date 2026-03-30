import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DistSupervisor } from '../../src/distributed/dist_supervisor.js';
import type { DistSupervisorOpts, DistChildSpec } from '../../src/distributed/dist_supervisor.js';
import type { DistributedRuntime } from '../../src/distributed/distributed_runtime.js';
import type { DistributedPid } from '../../src/distributed/dpid.js';

// ---------------------------------------------------------------------------
// Mock runtime factory
// ---------------------------------------------------------------------------

function createMockRuntime(selfNode = 'alpha'): {
  runtime: DistributedRuntime;
  spawnCalls: Array<{ fn: unknown }>;
  spawnRemoteCalls: Array<{ node: string; module: string; fn: string; args: unknown[] }>;
  monitorRemoteCalls: Array<{ watcherPid: string; pid: DistributedPid }>;
  failNodes: Set<string>;
} {
  let nextPid = 0;
  const spawnCalls: Array<{ fn: unknown }> = [];
  const spawnRemoteCalls: Array<{ node: string; module: string; fn: string; args: unknown[] }> = [];
  const monitorRemoteCalls: Array<{ watcherPid: string; pid: DistributedPid }> = [];
  const failNodes = new Set<string>();

  const runtime = {
    selfNode: () => selfNode,
    spawn: vi.fn((fn: unknown): DistributedPid => {
      spawnCalls.push({ fn });
      const pid: DistributedPid = { node: selfNode, local: `pid-${nextPid++}` };
      return pid;
    }),
    spawnRemote: vi.fn(async (node: string, module: string, fn: string, args: unknown[]): Promise<DistributedPid> => {
      if (failNodes.has(node)) {
        throw new Error(`Cannot reach node ${node}`);
      }
      spawnRemoteCalls.push({ node, module, fn, args });
      const pid: DistributedPid = { node, local: `remote-pid-${nextPid++}` };
      return pid;
    }),
    monitorRemote: vi.fn((watcherPid: string, pid: DistributedPid) => {
      monitorRemoteCalls.push({ watcherPid, pid });
    }),
  } as unknown as DistributedRuntime;

  return { runtime, spawnCalls, spawnRemoteCalls, monitorRemoteCalls, failNodes };
}

function makeLocalSpec(id: string, restart: 'permanent' | 'transient' | 'temporary' = 'permanent', node = 'alpha'): DistChildSpec {
  return {
    id,
    node,
    start: vi.fn(async () => {}),
    restart,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('DistSupervisor', () => {
  let mock: ReturnType<typeof createMockRuntime>;

  beforeEach(() => {
    mock = createMockRuntime('alpha');
  });

  it('starts supervisor with local children', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 5,
      maxSeconds: 60,
      children: [
        makeLocalSpec('child-a'),
        makeLocalSpec('child-b'),
      ],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();

    const children = sup.getChildren();
    expect(children.size).toBe(2);
    expect(children.get('child-a')?.pid).toBeDefined();
    expect(children.get('child-b')?.pid).toBeDefined();
    expect(children.get('child-a')!.pid!.node).toBe('alpha');
    expect(children.get('child-b')!.pid!.node).toBe('alpha');
    expect(mock.spawnCalls).toHaveLength(2);
  });

  it('one_for_one: restarts only the crashed child', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [
        makeLocalSpec('child-a'),
        makeLocalSpec('child-b'),
      ],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();

    const pidBBefore = sup.getChildren().get('child-b')!.pid;
    mock.spawnCalls.length = 0; // reset

    await sup.handleChildExit('child-a', 'crash');

    // Only one spawn call (child-a restarted)
    expect(mock.spawnCalls).toHaveLength(1);
    // child-b pid unchanged
    expect(sup.getChildren().get('child-b')!.pid).toEqual(pidBBefore);
  });

  it('all_for_one: restarts all children on crash', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'all_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [
        makeLocalSpec('child-a'),
        makeLocalSpec('child-b'),
        makeLocalSpec('child-c'),
      ],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    mock.spawnCalls.length = 0;

    await sup.handleChildExit('child-b', 'crash');

    // All three should be restarted
    expect(mock.spawnCalls).toHaveLength(3);
  });

  it('rest_for_one: restarts crashed child and those after it', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'rest_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [
        makeLocalSpec('child-a'),
        makeLocalSpec('child-b'),
        makeLocalSpec('child-c'),
      ],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    mock.spawnCalls.length = 0;

    await sup.handleChildExit('child-b', 'crash');

    // child-b and child-c should be restarted (not child-a)
    expect(mock.spawnCalls).toHaveLength(2);
    // child-a should still have its original pid
    const pidA = sup.getChildren().get('child-a')!.pid;
    expect(pidA).toBeDefined();
  });

  it('permanent: always restarts', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [makeLocalSpec('child-a', 'permanent')],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    mock.spawnCalls.length = 0;

    // Normal exit
    await sup.handleChildExit('child-a', 'normal');
    expect(mock.spawnCalls).toHaveLength(1);

    mock.spawnCalls.length = 0;

    // Abnormal exit
    await sup.handleChildExit('child-a', 'crash');
    expect(mock.spawnCalls).toHaveLength(1);
  });

  it('transient: restarts on abnormal exit only', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [makeLocalSpec('child-a', 'transient')],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    mock.spawnCalls.length = 0;

    // Normal exit: should NOT restart
    await sup.handleChildExit('child-a', 'normal');
    expect(mock.spawnCalls).toHaveLength(0);
    expect(sup.getChildren().get('child-a')!.pid).toBeNull();
  });

  it('transient: restarts on abnormal exit', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [makeLocalSpec('child-a', 'transient')],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    mock.spawnCalls.length = 0;

    await sup.handleChildExit('child-a', 'crash');
    expect(mock.spawnCalls).toHaveLength(1);
  });

  it('temporary: never restarts', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [makeLocalSpec('child-a', 'temporary')],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    mock.spawnCalls.length = 0;

    await sup.handleChildExit('child-a', 'crash');
    expect(mock.spawnCalls).toHaveLength(0);
    expect(sup.getChildren().get('child-a')!.pid).toBeNull();
  });

  it('restart intensity limit shuts down supervisor', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 2,
      maxSeconds: 60,
      children: [makeLocalSpec('child-a')],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();

    // First restart: ok (restartLog = 1)
    await sup.handleChildExit('child-a', 'crash');
    expect(sup.isRunning()).toBe(true);

    // Second restart: ok (restartLog = 2, <= maxRestarts)
    await sup.handleChildExit('child-a', 'crash');
    expect(sup.isRunning()).toBe(true);

    // Third restart: exceeds limit (restartLog = 3, > 2)
    await sup.handleChildExit('child-a', 'crash');
    expect(sup.isRunning()).toBe(false);
    expect(sup.getChildren().size).toBe(0);
  });

  it('handleNodeDown restarts affected children', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [
        makeLocalSpec('local-child'),
        makeLocalSpec('remote-child', 'permanent', 'beta'),
      ],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();

    // remote-child is on beta
    expect(sup.getChildren().get('remote-child')!.pid!.node).toBe('beta');

    mock.spawnCalls.length = 0;
    mock.spawnRemoteCalls.length = 0;

    await sup.handleNodeDown('beta');

    // remote-child should have been restarted on beta
    expect(mock.spawnRemoteCalls).toHaveLength(1);
    expect(mock.spawnRemoteCalls[0].node).toBe('beta');
    // local-child should NOT have been touched
    expect(mock.spawnCalls).toHaveLength(0);
  });

  it('fallback node when primary is unavailable', async () => {
    mock.failNodes.add('beta');

    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 10,
      maxSeconds: 60,
      children: [{
        id: 'remote-child',
        node: 'beta',
        fallbackNode: 'alpha',
        start: vi.fn(async () => {}),
        restart: 'permanent',
      }],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();

    // Should have spawned locally since beta is unavailable and fallback is alpha
    const child = sup.getChildren().get('remote-child');
    expect(child).toBeDefined();
    expect(child!.pid!.node).toBe('alpha');
    expect(mock.spawnCalls).toHaveLength(1);
  });

  it('shutdown clears all children', async () => {
    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 5,
      maxSeconds: 60,
      children: [
        makeLocalSpec('child-a'),
        makeLocalSpec('child-b'),
      ],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();
    expect(sup.getChildren().size).toBe(2);

    await sup.shutdown();
    expect(sup.getChildren().size).toBe(0);
    expect(sup.isRunning()).toBe(false);
  });

  it('getChildren returns correct state', async () => {
    const specA = makeLocalSpec('child-a');
    const specB = makeLocalSpec('child-b', 'transient');

    const opts: DistSupervisorOpts = {
      strategy: 'one_for_one',
      maxRestarts: 5,
      maxSeconds: 60,
      children: [specA, specB],
    };

    const sup = new DistSupervisor(mock.runtime, opts);
    await sup.start();

    const children = sup.getChildren();
    expect(children.size).toBe(2);

    const childA = children.get('child-a');
    expect(childA).toBeDefined();
    expect(childA!.spec.id).toBe('child-a');
    expect(childA!.spec.restart).toBe('permanent');
    expect(childA!.pid).not.toBeNull();

    const childB = children.get('child-b');
    expect(childB).toBeDefined();
    expect(childB!.spec.id).toBe('child-b');
    expect(childB!.spec.restart).toBe('transient');
    expect(childB!.pid).not.toBeNull();

    // Mutating returned map should not affect internal state
    children.delete('child-a');
    expect(sup.getChildren().size).toBe(2);
  });
});

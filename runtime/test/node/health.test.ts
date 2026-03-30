import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { HealthMonitor, DEFAULT_HEALTH_CONFIG } from "../../src/node/health.js";

describe("HealthMonitor", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function createMonitor(
    config: Record<string, number> = {},
    sendPing = vi.fn(),
    onNodeDown = vi.fn(),
  ) {
    return { monitor: new HealthMonitor(config, sendPing, onNodeDown), sendPing, onNodeDown };
  }

  it("starts monitoring a node", () => {
    const { monitor } = createMonitor();
    monitor.startMonitoring("node-a");
    const health = monitor.getHealth("node-a");
    expect(health).toBeDefined();
    expect(health!.status).toBe("up");
    expect(health!.missedPongs).toBe(0);
    monitor.shutdown();
  });

  it("receives pong and updates RTT", () => {
    const { monitor } = createMonitor();
    monitor.startMonitoring("node-a");

    // Advance past one ping interval so a ping is sent
    vi.advanceTimersByTime(DEFAULT_HEALTH_CONFIG.pingInterval);

    const healthBefore = monitor.getHealth("node-a");
    expect(healthBefore!.lastPingSent).toBeGreaterThan(0);

    // Simulate 10ms network latency then receive pong
    vi.advanceTimersByTime(10);
    monitor.receivePong("node-a");

    const healthAfter = monitor.getHealth("node-a");
    expect(healthAfter!.rtt).toBe(10);
    expect(healthAfter!.status).toBe("up");
    monitor.shutdown();
  });

  it("detects missed pongs", () => {
    const { monitor } = createMonitor({ pingInterval: 1000, pongTimeout: 2500, maxMissedPongs: 3 });
    monitor.startMonitoring("node-a");

    // Advance past pongTimeout + one ping interval so the check fires
    vi.advanceTimersByTime(1000); // first ping, no timeout yet
    vi.advanceTimersByTime(2000); // second ping, pongTimeout exceeded -> missedPongs=1

    const health = monitor.getHealth("node-a");
    expect(health!.missedPongs).toBeGreaterThanOrEqual(1);
    monitor.shutdown();
  });

  it("marks node as suspect after timeout", () => {
    const { monitor } = createMonitor({ pingInterval: 1000, pongTimeout: 1500, maxMissedPongs: 5 });
    monitor.startMonitoring("node-a");

    // First ping at 1000ms, at 2000ms pongTimeout (1500) exceeded -> suspect
    vi.advanceTimersByTime(2000);

    const health = monitor.getHealth("node-a");
    expect(health!.status).toBe("suspect");
    monitor.shutdown();
  });

  it("calls onNodeDown after maxMissedPongs", () => {
    const onNodeDown = vi.fn();
    const monitor = new HealthMonitor(
      { pingInterval: 1000, pongTimeout: 500, maxMissedPongs: 2 },
      vi.fn(),
      onNodeDown,
    );
    monitor.startMonitoring("node-a");

    // tick 1000: pong timeout exceeded (500 < 1000), missedPongs=1, status=suspect
    vi.advanceTimersByTime(1000);
    // tick 2000: missedPongs=2 >= maxMissedPongs, status=down, onNodeDown called
    vi.advanceTimersByTime(1000);

    expect(onNodeDown).toHaveBeenCalledWith("node-a");
    expect(monitor.getHealth("node-a")).toBeUndefined(); // removed after down
    monitor.shutdown();
  });

  it("resets missed count on pong", () => {
    const { monitor } = createMonitor({ pingInterval: 1000, pongTimeout: 500, maxMissedPongs: 5 });
    monitor.startMonitoring("node-a");

    // Let it miss one
    vi.advanceTimersByTime(1000);
    expect(monitor.getHealth("node-a")!.missedPongs).toBe(1);

    // Receive pong
    monitor.receivePong("node-a");
    expect(monitor.getHealth("node-a")!.missedPongs).toBe(0);
    expect(monitor.getHealth("node-a")!.status).toBe("up");
    monitor.shutdown();
  });

  it("stops monitoring a node", () => {
    const { monitor, sendPing } = createMonitor();
    monitor.startMonitoring("node-a");
    monitor.stopMonitoring("node-a");

    expect(monitor.getHealth("node-a")).toBeUndefined();

    // Ensure no more pings are sent
    sendPing.mockClear();
    vi.advanceTimersByTime(DEFAULT_HEALTH_CONFIG.pingInterval * 3);
    expect(sendPing).not.toHaveBeenCalled();
    monitor.shutdown();
  });

  it("handles multiple nodes", () => {
    const { monitor, sendPing } = createMonitor();
    monitor.startMonitoring("node-a");
    monitor.startMonitoring("node-b");

    vi.advanceTimersByTime(DEFAULT_HEALTH_CONFIG.pingInterval);

    expect(sendPing).toHaveBeenCalledWith("node-a");
    expect(sendPing).toHaveBeenCalledWith("node-b");

    const all = monitor.getAllHealth();
    expect(all.size).toBe(2);
    expect(all.has("node-a")).toBe(true);
    expect(all.has("node-b")).toBe(true);
    monitor.shutdown();
  });

  it("uses custom config", () => {
    const sendPing = vi.fn();
    const monitor = new HealthMonitor({ pingInterval: 2000 }, sendPing, vi.fn());
    monitor.startMonitoring("node-a");

    // At default 5000ms interval no ping would have fired, but at 2000 it should
    vi.advanceTimersByTime(2000);
    expect(sendPing).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(2000);
    expect(sendPing).toHaveBeenCalledTimes(2);
    monitor.shutdown();
  });

  it("shutdown clears all timers", () => {
    const { monitor, sendPing } = createMonitor();
    monitor.startMonitoring("node-a");
    monitor.startMonitoring("node-b");
    monitor.shutdown();

    sendPing.mockClear();
    vi.advanceTimersByTime(DEFAULT_HEALTH_CONFIG.pingInterval * 5);
    expect(sendPing).not.toHaveBeenCalled();
    expect(monitor.getAllHealth().size).toBe(0);
  });

  it("receivePong on unknown node is a no-op", () => {
    const { monitor } = createMonitor();
    // Should not throw
    monitor.receivePong("unknown-node");
    expect(monitor.getHealth("unknown-node")).toBeUndefined();
    monitor.shutdown();
  });

  it("sends ping on each interval tick", () => {
    const sendPing = vi.fn();
    const monitor = new HealthMonitor({ pingInterval: 1000 }, sendPing, vi.fn());
    monitor.startMonitoring("node-a");

    vi.advanceTimersByTime(3000);
    expect(sendPing).toHaveBeenCalledTimes(3);
    monitor.shutdown();
  });
});

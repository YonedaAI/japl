export interface HealthConfig {
  pingInterval: number; // ms between pings (default: 5000)
  pongTimeout: number; // ms to wait for pong (default: 15000)
  maxMissedPongs: number; // missed pongs before NODE_DOWN (default: 3)
}

export const DEFAULT_HEALTH_CONFIG: HealthConfig = {
  pingInterval: 5000,
  pongTimeout: 15000,
  maxMissedPongs: 3,
};

export interface NodeHealth {
  lastPingSent: number;
  lastPongReceived: number;
  missedPongs: number;
  rtt: number; // round-trip time in ms
  status: "up" | "suspect" | "down";
}

export class HealthMonitor {
  private nodes: Map<string, NodeHealth> = new Map();
  private timers: Map<string, ReturnType<typeof setInterval>> = new Map();
  private config: HealthConfig;
  private sendPing: (nodeName: string) => void;
  private onNodeDown: (nodeName: string) => void;

  constructor(
    config: Partial<HealthConfig>,
    sendPing: (nodeName: string) => void,
    onNodeDown: (nodeName: string) => void,
  ) {
    this.config = { ...DEFAULT_HEALTH_CONFIG, ...config };
    this.sendPing = sendPing;
    this.onNodeDown = onNodeDown;
  }

  /** Start monitoring a node. */
  startMonitoring(nodeName: string): void {
    const health: NodeHealth = {
      lastPingSent: 0,
      lastPongReceived: Date.now(),
      missedPongs: 0,
      rtt: 0,
      status: "up",
    };
    this.nodes.set(nodeName, health);
    this.schedulePing(nodeName);
  }

  /** Stop monitoring a node. */
  stopMonitoring(nodeName: string): void {
    const timer = this.timers.get(nodeName);
    if (timer) clearInterval(timer);
    this.timers.delete(nodeName);
    this.nodes.delete(nodeName);
  }

  /** Called when we receive a PONG from a node. */
  receivePong(nodeName: string): void {
    const health = this.nodes.get(nodeName);
    if (!health) return;
    health.lastPongReceived = Date.now();
    health.rtt = health.lastPongReceived - health.lastPingSent;
    health.missedPongs = 0;
    health.status = "up";
  }

  /** Get health status of a node. */
  getHealth(nodeName: string): NodeHealth | undefined {
    return this.nodes.get(nodeName);
  }

  /** Get all monitored nodes. */
  getAllHealth(): Map<string, NodeHealth> {
    return new Map(this.nodes);
  }

  private schedulePing(nodeName: string): void {
    const timer = setInterval(() => {
      const health = this.nodes.get(nodeName);
      if (!health) return;

      // Check if previous pong was received
      const timeSinceLastPong = Date.now() - health.lastPongReceived;
      if (timeSinceLastPong > this.config.pongTimeout) {
        health.missedPongs++;
        if (health.missedPongs >= this.config.maxMissedPongs) {
          health.status = "down";
          this.stopMonitoring(nodeName);
          this.onNodeDown(nodeName);
          return;
        }
        health.status = "suspect";
      }

      // Send ping
      health.lastPingSent = Date.now();
      this.sendPing(nodeName);
    }, this.config.pingInterval);

    this.timers.set(nodeName, timer);
  }

  /** Shut down all monitoring. */
  shutdown(): void {
    for (const [name] of this.timers) {
      this.stopMonitoring(name);
    }
  }
}

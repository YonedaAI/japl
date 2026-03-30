// Exponential backoff reconnection for JAPL node connections.

export class Reconnector {
  private attempts: number = 0;
  private maxDelay: number = 30000; // 30 seconds max
  private baseDelay: number = 1000; // 1 second start
  private timer: ReturnType<typeof setTimeout> | null = null;
  private stopped: boolean = false;

  constructor(private connectFn: () => Promise<void>) {}

  start(): void {
    this.stopped = false;
    this.attempt();
  }

  stop(): void {
    this.stopped = true;
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  reset(): void {
    this.attempts = 0;
  }

  getAttempts(): number {
    return this.attempts;
  }

  getNextDelay(): number {
    return Math.min(this.baseDelay * Math.pow(2, this.attempts), this.maxDelay);
  }

  private async attempt(): Promise<void> {
    if (this.stopped) return;
    try {
      await this.connectFn();
      this.attempts = 0; // reset on success
    } catch {
      if (this.stopped) return;
      this.attempts++;
      const delay = Math.min(this.baseDelay * Math.pow(2, this.attempts - 1), this.maxDelay);
      this.timer = setTimeout(() => this.attempt(), delay);
    }
  }
}

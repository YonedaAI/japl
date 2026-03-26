import type { Option } from "./types.js";
import { Some, None } from "./types.js";

export class Mailbox<T> {
  private queue: T[] = [];
  private waiters: ((msg: T) => void)[] = [];

  send(msg: T): void {
    if (this.waiters.length > 0) {
      const waiter = this.waiters.shift()!;
      waiter(msg);
    } else {
      this.queue.push(msg);
    }
  }

  receive(): Promise<T> {
    if (this.queue.length > 0) {
      return Promise.resolve(this.queue.shift()!);
    }
    return new Promise(resolve => this.waiters.push(resolve));
  }

  receiveTimeout(ms: number): Promise<Option<T>> {
    if (this.queue.length > 0) {
      return Promise.resolve(Some(this.queue.shift()!));
    }
    return new Promise(resolve => {
      let settled = false;
      const timer = setTimeout(() => {
        if (!settled) {
          settled = true;
          // Remove this waiter from the list
          const idx = this.waiters.indexOf(waiter);
          if (idx !== -1) this.waiters.splice(idx, 1);
          resolve(None);
        }
      }, ms);

      const waiter = (msg: T) => {
        if (!settled) {
          settled = true;
          clearTimeout(timer);
          resolve(Some(msg));
        }
      };

      this.waiters.push(waiter);
    });
  }

  selectiveReceive(predicate: (msg: T) => boolean): Promise<T> {
    // Check queue for matching message first
    for (let i = 0; i < this.queue.length; i++) {
      if (predicate(this.queue[i])) {
        return Promise.resolve(this.queue.splice(i, 1)[0]);
      }
    }

    // Otherwise wait for a matching message (save non-matching to queue)
    return new Promise(resolve => {
      const waiter = (msg: T) => {
        if (predicate(msg)) {
          resolve(msg);
        } else {
          // Put non-matching message back in queue
          this.queue.push(msg);
          // Re-register waiter
          this.waiters.push(waiter);
        }
      };
      this.waiters.push(waiter);
    });
  }

  get length(): number {
    return this.queue.length;
  }
}

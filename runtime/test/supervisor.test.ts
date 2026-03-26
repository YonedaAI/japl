import { describe, it, expect } from "vitest";
import { Supervisor } from "../src/supervisor.js";
import { Scheduler } from "../src/scheduler.js";
import type { ProcessContext } from "../src/process.js";
import type { ChildSpec, SupervisorOpts } from "../src/supervisor.js";

function delay(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

describe("Supervisor", () => {
  function makeChild(scheduler: Scheduler, id: string, shouldCrash = false): ChildSpec {
    return {
      id,
      start: () =>
        scheduler.spawn(async (ctx: ProcessContext) => {
          if (shouldCrash) {
            throw new Error(`${id} crashed`);
          }
          // Normal child: wait for messages
          await ctx.mailbox.receive();
        }),
      restart: "permanent",
    };
  }

  it("starts supervisor with 3 children", async () => {
    const scheduler = new Scheduler();
    const opts: SupervisorOpts = {
      strategy: "one_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        makeChild(scheduler, "child1"),
        makeChild(scheduler, "child2"),
        makeChild(scheduler, "child3"),
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(20);

    expect(sup.getChildren().size).toBe(3);
    expect(sup.getChildren().has("child1")).toBe(true);
    expect(sup.getChildren().has("child2")).toBe(true);
    expect(sup.getChildren().has("child3")).toBe(true);
  });

  it("one_for_one: only crashed child restarts", async () => {
    const scheduler = new Scheduler();
    let crashCount = 0;

    const opts: SupervisorOpts = {
      strategy: "one_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        {
          id: "crasher",
          start: () => {
            crashCount++;
            return scheduler.spawn(async () => {
              if (crashCount <= 1) {
                throw new Error("crash once");
              }
              await new Promise(() => {}); // hang forever
            });
          },
          restart: "permanent",
        },
        makeChild(scheduler, "stable"),
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(50);

    // crasher should have been restarted (crashCount >= 2)
    expect(crashCount).toBeGreaterThanOrEqual(2);
    // stable child should still be there with same entry
    expect(sup.getChildren().has("stable")).toBe(true);
  });

  it("all_for_one: all children restart on one crash", async () => {
    const scheduler = new Scheduler();
    let child1Starts = 0;
    let child2Starts = 0;

    const opts: SupervisorOpts = {
      strategy: "all_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        {
          id: "crasher",
          start: () => {
            child1Starts++;
            return scheduler.spawn(async () => {
              if (child1Starts <= 1) {
                throw new Error("crash");
              }
              await new Promise(() => {});
            });
          },
          restart: "permanent",
        },
        {
          id: "other",
          start: () => {
            child2Starts++;
            return scheduler.spawn(async (ctx: ProcessContext) => {
              await ctx.mailbox.receive();
            });
          },
          restart: "permanent",
        },
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(50);

    // Both should have been started more than once
    expect(child1Starts).toBeGreaterThanOrEqual(2);
    expect(child2Starts).toBeGreaterThanOrEqual(2);
  });

  it("permanent: always restarts", async () => {
    const scheduler = new Scheduler();
    let starts = 0;

    const opts: SupervisorOpts = {
      strategy: "one_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        {
          id: "perm",
          start: () => {
            starts++;
            return scheduler.spawn(async () => {
              if (starts <= 2) {
                throw new Error("crash");
              }
              await new Promise(() => {});
            });
          },
          restart: "permanent",
        },
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(100);

    expect(starts).toBeGreaterThanOrEqual(3);
  });

  it("transient: restarts only on abnormal exit", async () => {
    const scheduler = new Scheduler();
    let starts = 0;

    const opts: SupervisorOpts = {
      strategy: "one_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        {
          id: "trans",
          start: () => {
            starts++;
            return scheduler.spawn(async () => {
              // Normal exit — should not be restarted
            });
          },
          restart: "transient",
        },
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(50);

    // Should only start once since it exited normally
    expect(starts).toBe(1);
  });

  it("temporary: never restarts", async () => {
    const scheduler = new Scheduler();
    let starts = 0;

    const opts: SupervisorOpts = {
      strategy: "one_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        {
          id: "temp",
          start: () => {
            starts++;
            return scheduler.spawn(async () => {
              throw new Error("crash");
            });
          },
          restart: "temporary",
        },
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(50);

    // Should only start once even though it crashed
    expect(starts).toBe(1);
  });

  it("restart intensity limit stops supervisor", async () => {
    const scheduler = new Scheduler();
    let starts = 0;

    const opts: SupervisorOpts = {
      strategy: "one_for_one",
      maxRestarts: 2,
      maxSeconds: 10,
      children: [
        {
          id: "crasher",
          start: () => {
            starts++;
            return scheduler.spawn(async () => {
              throw new Error("always crash");
            });
          },
          restart: "permanent",
        },
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(100);

    // Should not restart indefinitely due to intensity limit
    // maxRestarts=2 means at most 2 restarts + 1 initial = 3 starts
    expect(starts).toBeLessThanOrEqual(4);
  });

  it("rest_for_one: restarts crashed child and children after it", async () => {
    const scheduler = new Scheduler();
    let child1Starts = 0;
    let child2Starts = 0;
    let child3Starts = 0;

    const opts: SupervisorOpts = {
      strategy: "rest_for_one",
      maxRestarts: 5,
      maxSeconds: 10,
      children: [
        {
          id: "first",
          start: () => {
            child1Starts++;
            return scheduler.spawn(async (ctx: ProcessContext) => {
              await ctx.mailbox.receive();
            });
          },
          restart: "permanent",
        },
        {
          id: "crasher",
          start: () => {
            child2Starts++;
            return scheduler.spawn(async () => {
              if (child2Starts <= 1) {
                throw new Error("crash");
              }
              await new Promise(() => {});
            });
          },
          restart: "permanent",
        },
        {
          id: "last",
          start: () => {
            child3Starts++;
            return scheduler.spawn(async (ctx: ProcessContext) => {
              await ctx.mailbox.receive();
            });
          },
          restart: "permanent",
        },
      ],
    };

    const sup = new Supervisor(opts, scheduler);
    sup.start();
    await delay(50);

    // first child should NOT be restarted
    expect(child1Starts).toBe(1);
    // crasher and last should be restarted
    expect(child2Starts).toBeGreaterThanOrEqual(2);
    expect(child3Starts).toBeGreaterThanOrEqual(2);
  });
});

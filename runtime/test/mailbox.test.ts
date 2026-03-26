import { describe, it, expect } from "vitest";
import { Mailbox } from "../src/mailbox.js";

describe("Mailbox", () => {
  it("send then receive returns message immediately", async () => {
    const mb = new Mailbox<string>();
    mb.send("hello");
    const msg = await mb.receive();
    expect(msg).toBe("hello");
  });

  it("receive then send resolves when message arrives", async () => {
    const mb = new Mailbox<string>();
    const promise = mb.receive();
    mb.send("delayed");
    const msg = await promise;
    expect(msg).toBe("delayed");
  });

  it("maintains FIFO order for multiple messages", async () => {
    const mb = new Mailbox<number>();
    mb.send(1);
    mb.send(2);
    mb.send(3);
    expect(await mb.receive()).toBe(1);
    expect(await mb.receive()).toBe(2);
    expect(await mb.receive()).toBe(3);
  });

  it("selective receive finds matching message in queue", async () => {
    const mb = new Mailbox<{ type: string; value: number }>();
    mb.send({ type: "a", value: 1 });
    mb.send({ type: "b", value: 2 });
    mb.send({ type: "a", value: 3 });

    const msg = await mb.selectiveReceive(m => m.type === "b");
    expect(msg).toEqual({ type: "b", value: 2 });
  });

  it("selective receive waits for matching message", async () => {
    const mb = new Mailbox<{ type: string; value: number }>();
    const promise = mb.selectiveReceive(m => m.type === "b");

    mb.send({ type: "a", value: 1 });
    mb.send({ type: "b", value: 2 });

    const msg = await promise;
    expect(msg).toEqual({ type: "b", value: 2 });
    // Non-matching message should still be in queue
    expect(mb.length).toBe(1);
  });

  it("receiveTimeout returns Some when message arrives in time", async () => {
    const mb = new Mailbox<string>();
    mb.send("fast");
    const result = await mb.receiveTimeout(100);
    expect(result._tag).toBe("Some");
    if (result._tag === "Some") {
      expect(result.value).toBe("fast");
    }
  });

  it("receiveTimeout returns None on timeout", async () => {
    const mb = new Mailbox<string>();
    const result = await mb.receiveTimeout(10);
    expect(result._tag).toBe("None");
  });

  it("reports correct length", () => {
    const mb = new Mailbox<number>();
    expect(mb.length).toBe(0);
    mb.send(1);
    mb.send(2);
    expect(mb.length).toBe(2);
  });

  it("receiveTimeout returns Some when message arrives before timeout", async () => {
    const mb = new Mailbox<string>();
    const promise = mb.receiveTimeout(200);
    setTimeout(() => mb.send("arrived"), 10);
    const result = await promise;
    expect(result._tag).toBe("Some");
    if (result._tag === "Some") {
      expect(result.value).toBe("arrived");
    }
  });
});

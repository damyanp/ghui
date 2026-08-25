import { describe, expect, it, vi } from "vitest";
import { SerializedTaskQueue } from "./serializedTaskQueue";

describe("SerializedTaskQueue", () => {
  it("runs tasks in enqueue order and flushes the latest task", async () => {
    const queue = new SerializedTaskQueue();
    const events: string[] = [];
    let releaseFirst: (() => void) | undefined;
    const firstBlocked = new Promise<void>((resolve) => {
      releaseFirst = resolve;
    });

    queue.enqueue(async () => {
      events.push("first-started");
      await firstBlocked;
      events.push("first-finished");
    }, vi.fn());
    queue.enqueue(async () => {
      events.push("second");
    }, vi.fn());

    await Promise.resolve();
    expect(events).toEqual(["first-started"]);
    releaseFirst?.();
    await queue.flush();
    expect(events).toEqual(["first-started", "first-finished", "second"]);
  });

  it("continues after a failed task", async () => {
    const queue = new SerializedTaskQueue();
    const onError = vi.fn();
    const second = vi.fn(async () => {});

    queue.enqueue(async () => {
      throw new Error("failed");
    }, onError);
    queue.enqueue(second, onError);
    await queue.flush();

    expect(onError).toHaveBeenCalledOnce();
    expect(second).toHaveBeenCalledOnce();
  });
});

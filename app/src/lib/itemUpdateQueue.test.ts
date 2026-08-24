import { afterEach, describe, expect, it, vi } from "vitest";
import { ItemUpdateQueue } from "./itemUpdateQueue";

describe("ItemUpdateQueue", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("cancels queued items when cleared", async () => {
    vi.useFakeTimers();
    const submit = vi.fn(async (_items: string[]) => {});
    const queue = new ItemUpdateQueue(submit, () => {});
    queue.add("old-account-item");

    queue.clear();
    await vi.runAllTimersAsync();

    expect(submit).not.toHaveBeenCalled();
  });

  it("submits items added to the same turn as one batch", async () => {
    vi.useFakeTimers();
    const submit = vi.fn(async (_items: string[]) => {});
    const queue = new ItemUpdateQueue(submit, () => {});
    queue.add("first");
    queue.add("second");

    await vi.runAllTimersAsync();

    expect(submit).toHaveBeenCalledWith(["first", "second"]);
  });
});

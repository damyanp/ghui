import { describe, expect, it } from "vitest";
import { mergePendingExtraData } from "./workItemExtraData";

describe("mergePendingExtraData", () => {
  it("preserves edits made while account data is loading", () => {
    const loaded = {
      unchanged: { note: "loaded" },
      edited: { note: "stale" },
    };
    const pending = new Map([
      ["edited", { note: "new edit" }],
      ["added", { note: "new item" }],
    ]);

    expect(mergePendingExtraData(loaded, pending)).toEqual({
      unchanged: { note: "loaded" },
      edited: { note: "new edit" },
      added: { note: "new item" },
    });
  });
});

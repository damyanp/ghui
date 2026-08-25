import { describe, expect, it } from "vitest";
import { canSaveWorkItemExtraDataEditor } from "./workItemExtraDataEditor";

const first = { host: "github.com", login: "first" };

describe("canSaveWorkItemExtraDataEditor", () => {
  it("allows an editor from the current account generation", () => {
    expect(canSaveWorkItemExtraDataEditor(first, first, 3, 3)).toBe(true);
  });

  it("rejects an editor after an A to B to A account cycle", () => {
    expect(canSaveWorkItemExtraDataEditor(first, first, 3, 5)).toBe(false);
  });

  it("rejects an editor for another identity", () => {
    expect(
      canSaveWorkItemExtraDataEditor(
        first,
        { host: "github.com", login: "second" },
        3,
        3
      )
    ).toBe(false);
  });
});

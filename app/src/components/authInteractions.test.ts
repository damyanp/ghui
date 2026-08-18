import { describe, expect, it } from "vitest";
import { isDirectAuthSwitch } from "./authInteractions";

describe("isDirectAuthSwitch", () => {
  it("returns false for a normal click", () => {
    expect(isDirectAuthSwitch({ ctrlKey: false })).toBe(false);
  });

  it("returns true for a control-click", () => {
    expect(isDirectAuthSwitch({ ctrlKey: true })).toBe(true);
  });
});

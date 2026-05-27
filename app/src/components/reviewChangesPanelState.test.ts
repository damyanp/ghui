import { describe, expect, it } from "vitest";
import { getInitialActiveTab } from "./reviewChangesPanelState";

describe("getInitialActiveTab", () => {
  it("defaults to pending changes tab when both lists are empty", () => {
    expect(getInitialActiveTab(0, 0)).toBe("changes");
  });

  it("defaults to conflicts tab when only conflicts exist", () => {
    expect(getInitialActiveTab(0, 2)).toBe("conflicts");
  });

  it("honors a preferred pending tab when pending changes exist", () => {
    expect(getInitialActiveTab(3, 2, "changes")).toBe("changes");
  });

  it("falls back to conflicts when pending is preferred but empty", () => {
    expect(getInitialActiveTab(0, 2, "changes")).toBe("conflicts");
  });

  it("honors a preferred conflicts tab when conflicts exist", () => {
    expect(getInitialActiveTab(3, 2, "conflicts")).toBe("conflicts");
  });

  it("falls back to pending when conflicts is preferred but empty", () => {
    expect(getInitialActiveTab(3, 0, "conflicts")).toBe("changes");
  });
});

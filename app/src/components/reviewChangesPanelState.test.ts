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

  it("honors a preferred conflicts tab when conflicts exist", () => {
    expect(getInitialActiveTab(3, 2, "conflicts")).toBe("conflicts");
  });
});

import { describe, expect, it } from "vitest";
import type { PivotConfig } from "./bindings/PivotConfig";
import { PRESETS } from "./recipePresets";
import {
  applyPreset,
  applyRecipeText,
  format,
  parse,
  setToggle,
} from "./recipeText";

function makeConfig(): PivotConfig {
  return {
    recipe: [{ kind: "pivot", field: "epic" }, { kind: "hierarchy" }],
    multiValueStrategy: "combined",
    showGhostAncestors: true,
  };
}

describe("recipe text", () => {
  it("round-trips each preset", () => {
    for (const preset of PRESETS) {
      expect(format(parse(preset.recipe))).toBe(preset.recipe);
    }
  });

  it("updates value.recipe when a preset is picked", () => {
    const next = applyPreset(makeConfig(), PRESETS[4].recipe);
    expect(next.recipe).toEqual(parse(PRESETS[4].recipe));
  });

  it("updates value.recipe when apply parses typed text", () => {
    const next = applyRecipeText(makeConfig(), "Sort(Priority) -> Hierarchy");
    expect(format(next.recipe)).toBe("Sort(Priority) → Hierarchy");
  });
});

describe("recipe toggles", () => {
  it("sets multiValueStrategy to explode when explodeMulti is enabled", () => {
    expect(setToggle(makeConfig(), "explodeMulti", true).multiValueStrategy).toBe(
      "explode"
    );
  });

  it("sets multiValueStrategy to combined when explodeMulti is disabled", () => {
    expect(
      setToggle(
        { ...makeConfig(), multiValueStrategy: "explode" },
        "explodeMulti",
        false
      ).multiValueStrategy
    ).toBe("combined");
  });

  it("updates showGhostAncestors when that toggle changes", () => {
    expect(
      setToggle(makeConfig(), "showGhostAncestors", false).showGhostAncestors
    ).toBe(false);
  });
});

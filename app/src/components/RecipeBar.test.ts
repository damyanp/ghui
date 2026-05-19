import { describe, expect, it, vi } from "vitest";
import type { Axis } from "$lib/bindings/Axis";
import type { PivotConfig } from "$lib/bindings/PivotConfig";
import { applyText, getToggleChecked, setToggle } from "./recipeBarState";

function makeConfig(recipe: Axis[] = []): PivotConfig {
  return {
    recipe,
    multiValueStrategy: "combined",
    showGhostAncestors: true,
  };
}

const canonicalFormat = async (recipe: Axis[]): Promise<string> =>
  recipe
    .map((a) =>
      a.kind === "hierarchy"
        ? "Hierarchy"
        : `${a.kind[0].toUpperCase() + a.kind.slice(1)}(${a.field})`
    )
    .join(" → ");

describe("applyText — error paths", () => {
  it("surfaces an error when the parser rejects an unknown field name", async () => {
    const parse = vi.fn().mockResolvedValue({
      ok: false,
      error: "Unknown field: NotAField",
    });

    const result = await applyText(
      "Pivot(NotAField)",
      makeConfig(),
      parse,
      canonicalFormat
    );

    expect(result).toEqual({ ok: false, error: "Unknown field: NotAField" });
    expect(parse).toHaveBeenCalledWith("Pivot(NotAField)");
  });

  it("surfaces an error when the parser rejects an unknown axis name", async () => {
    const parse = vi.fn().mockResolvedValue({
      ok: false,
      error: "Unknown axis: Explode (use Pivot, Group, Sort, or Hierarchy)",
    });

    const result = await applyText(
      "Explode(Epic)",
      makeConfig(),
      parse,
      canonicalFormat
    );

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain("Unknown axis");
    }
  });

  it("surfaces an error when a required argument is missing", async () => {
    const parse = vi.fn().mockResolvedValue({
      ok: false,
      error: "pivot requires a field argument, e.g. pivot(Epic)",
    });

    const result = await applyText(
      "Pivot",
      makeConfig(),
      parse,
      canonicalFormat
    );

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain("requires a field argument");
    }
  });
});

describe("applyText — alias normalisation", () => {
  it("passes the raw alias text to parse and uses the canonical field from the result", async () => {
    const normalised: Axis[] = [{ kind: "pivot", field: "issueType" }];
    const parse = vi.fn().mockResolvedValue({ ok: true, recipe: normalised });
    const format = vi
      .fn()
      .mockResolvedValue("Pivot(IssueType)");

    const result = await applyText(
      "Pivot(Issue_Type)",
      makeConfig(),
      parse,
      format
    );

    // parse must have been called with the raw alias text
    expect(parse).toHaveBeenCalledWith("Pivot(Issue_Type)");

    // result contains the Rust-normalised canonical field
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.config.recipe).toEqual(normalised);
      expect(result.config.recipe[0]).toMatchObject({
        kind: "pivot",
        field: "issueType",
      });
      expect(result.formattedText).toBe("Pivot(IssueType)");
    }
  });
});

describe("setToggle", () => {
  it("sets multiValueStrategy to explode when explodeMulti is enabled", () => {
    expect(
      setToggle(makeConfig(), "explodeMulti", true).multiValueStrategy
    ).toBe("explode");
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

describe("getToggleChecked — initial state reflects bound PivotConfig", () => {
  it("reports explodeMulti as checked when multiValueStrategy is 'explode'", () => {
    const config: PivotConfig = {
      ...makeConfig(),
      multiValueStrategy: "explode",
    };
    expect(getToggleChecked(config, "explodeMulti")).toBe(true);
  });

  it("reports explodeMulti as unchecked when multiValueStrategy is 'combined'", () => {
    const config: PivotConfig = {
      ...makeConfig(),
      multiValueStrategy: "combined",
    };
    expect(getToggleChecked(config, "explodeMulti")).toBe(false);
  });

  it("reports showGhostAncestors according to the bound config value", () => {
    expect(
      getToggleChecked(
        { ...makeConfig(), showGhostAncestors: true },
        "showGhostAncestors"
      )
    ).toBe(true);
    expect(
      getToggleChecked(
        { ...makeConfig(), showGhostAncestors: false },
        "showGhostAncestors"
      )
    ).toBe(false);
  });
});

describe("RecipeBar Explode checkbox — onApply wiring", () => {
  // These tests pin the wiring used in RecipeBar.svelte's `updateToggle`
  // handler: `emit(setToggle(value, toggle, checked))` where `emit` ends
  // by calling `onApply(next)`. Keeping the test free of Svelte runtime
  // per the repo convention (extract pure logic, test the helper).
  function simulateCheckboxChange(
    value: PivotConfig,
    toggle: Parameters<typeof setToggle>[1],
    checked: boolean,
    onApply: (cfg: PivotConfig) => void
  ): void {
    onApply(setToggle(value, toggle, checked));
  }

  it("calls onApply with multiValueStrategy: 'explode' when toggled on", () => {
    const onApply = vi.fn<(cfg: PivotConfig) => void>();
    const initial: PivotConfig = {
      ...makeConfig(),
      multiValueStrategy: "combined",
    };

    simulateCheckboxChange(initial, "explodeMulti", true, onApply);

    expect(onApply).toHaveBeenCalledTimes(1);
    expect(onApply).toHaveBeenCalledWith(
      expect.objectContaining({ multiValueStrategy: "explode" })
    );
  });

  it("calls onApply with multiValueStrategy: 'combined' when toggled off", () => {
    const onApply = vi.fn<(cfg: PivotConfig) => void>();
    const initial: PivotConfig = {
      ...makeConfig(),
      multiValueStrategy: "explode",
    };

    simulateCheckboxChange(initial, "explodeMulti", false, onApply);

    expect(onApply).toHaveBeenCalledTimes(1);
    expect(onApply).toHaveBeenCalledWith(
      expect.objectContaining({ multiValueStrategy: "combined" })
    );
  });

  it("preserves the rest of the PivotConfig when emitting the toggle change", () => {
    const onApply = vi.fn<(cfg: PivotConfig) => void>();
    const recipe: Axis[] = [{ kind: "pivot", field: "assignee" }];
    const initial: PivotConfig = {
      recipe,
      multiValueStrategy: "combined",
      showGhostAncestors: false,
    };

    simulateCheckboxChange(initial, "explodeMulti", true, onApply);

    expect(onApply).toHaveBeenCalledWith({
      recipe,
      multiValueStrategy: "explode",
      showGhostAncestors: false,
    });
  });
});

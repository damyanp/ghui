import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Axis } from "$lib/bindings/Axis";
import { parseRecipe, recipeToString } from "./recipeParser";
import fixtureData from "../../../github-graphql/tests/fixtures/recipes.json";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

describe("recipeParser", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("delegates parseRecipe to the Rust parser command", async () => {
    const expected: Axis[] = [
      { kind: "pivot", field: "epic" },
      { kind: "hierarchy" },
    ];
    invokeMock.mockResolvedValue(expected);

    const parsed = await parseRecipe("Pivot(Epic) -> Hierarchy");

    expect(invokeMock).toHaveBeenCalledWith("parse_recipe", {
      text: "Pivot(Epic) -> Hierarchy",
    });
    expect(parsed).toEqual({ ok: true, recipe: expected });
  });

  it("returns parse errors from the Rust parser command", async () => {
    invokeMock.mockRejectedValue("Unknown field: NotAField");

    const parsed = await parseRecipe("Pivot(NotAField)");

    expect(parsed).toEqual({
      ok: false,
      error: "Unknown field: NotAField",
    });
  });

  it("delegates recipeToString to the Rust formatter command", async () => {
    const recipe: Axis[] = [
      { kind: "pivot", field: "epic" },
      { kind: "hierarchy" },
    ];
    invokeMock.mockResolvedValue("Pivot(Epic) → Hierarchy");

    const text = await recipeToString(recipe);

    expect(invokeMock).toHaveBeenCalledWith("recipe_to_string", { recipe });
    expect(text).toBe("Pivot(Epic) → Hierarchy");
  });

  it("forwards every canonical fixture recipe to the parse_recipe command", async () => {
    const fixtures = fixtureData as Record<string, Axis[]>;

    for (const [text, axes] of Object.entries(fixtures)) {
      invokeMock.mockResolvedValueOnce(axes);

      const result = await parseRecipe(text);

      expect(invokeMock).toHaveBeenCalledWith("parse_recipe", { text });
      expect(result).toEqual({ ok: true, recipe: axes });
      invokeMock.mockReset();
    }
  });

  it("propagates errors thrown by recipeToString", async () => {
    invokeMock.mockRejectedValue(new Error("recipe_to_string failed"));

    await expect(recipeToString([])).rejects.toThrow(
      "recipe_to_string failed"
    );
  });
});

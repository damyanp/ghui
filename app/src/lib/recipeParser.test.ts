import { describe, expect, it } from "vitest";
import fixtures from "../../test-fixtures/recipes.json";
import { parseRecipe, recipeToString } from "./recipeParser";

describe("recipeParser fixture parity", () => {
  it("parses every fixture recipe into the expected axes", () => {
    for (const [input, expected] of Object.entries(fixtures)) {
      const parsed = parseRecipe(input);
      expect(parsed).toEqual({ ok: true, recipe: expected });
    }
  });

  it("stringifies parsed fixtures back to canonical text", () => {
    for (const [input] of Object.entries(fixtures)) {
      const parsed = parseRecipe(input);
      expect(parsed.ok).toBe(true);
      if (parsed.ok) {
        expect(recipeToString(parsed.recipe)).toBe(input);
      }
    }
  });
});

describe("recipeParser error messages", () => {
  it("reports unknown fields", () => {
    expect(parseRecipe("Pivot(NotAField)")).toEqual({
      ok: false,
      error: "Unknown field: NotAField",
    });
  });

  it("reports unknown axes", () => {
    expect(parseRecipe("Bucket(Epic)")).toEqual({
      ok: false,
      error: "Unknown axis: Bucket(Epic) (use Pivot, Group, Sort, or Hierarchy)",
    });
  });

  it("reports malformed axis syntax", () => {
    expect(parseRecipe("Pivot(Epic")).toEqual({
      ok: false,
      error: 'Could not parse axis: "Pivot(Epic"',
    });
  });
});

describe("recipeParser compatibility behavior", () => {
  it("ignores dangling separators", () => {
    expect(parseRecipe("Pivot(Epic) ->")).toEqual({
      ok: true,
      recipe: [{ kind: "pivot", field: "epic" }],
    });
  });

  it("accepts field aliases", () => {
    expect(parseRecipe("Pivot(Repo) -> Group(Owner) -> Sort(Assignees)")).toEqual({
      ok: true,
      recipe: [
        { kind: "pivot", field: "repository" },
        { kind: "group", field: "assignee" },
        { kind: "sort", field: "assignee" },
      ],
    });
  });
});

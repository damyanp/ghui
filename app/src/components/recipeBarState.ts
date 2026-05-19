import type { Axis } from "$lib/bindings/Axis";
import type { PivotConfig } from "$lib/bindings/PivotConfig";

export type RecipeBarToggle = "explodeMulti" | "showGhostAncestors";

export type ParseFn = (
  text: string
) => Promise<{ ok: true; recipe: Axis[] } | { ok: false; error: string }>;

export type FormatFn = (recipe: Axis[]) => Promise<string>;

export type ApplyTextResult =
  | { ok: true; config: PivotConfig; formattedText: string }
  | { ok: false; error: string };

/**
 * Parse `text` into a new PivotConfig, returning a canonical formatted string
 * alongside. Dependencies are injected so this function is testable without
 * Tauri or Svelte runtimes.
 */
export async function applyText(
  text: string,
  config: PivotConfig,
  parse: ParseFn,
  format: FormatFn
): Promise<ApplyTextResult> {
  const result = await parse(text);
  if (!result.ok) {
    return { ok: false, error: result.error };
  }
  const next: PivotConfig = { ...config, recipe: result.recipe };
  const formattedText = await format(result.recipe);
  return { ok: true, config: next, formattedText };
}

/**
 * Apply a RecipeBar toggle to a PivotConfig, returning the updated config.
 */
export function setToggle(
  config: PivotConfig,
  toggle: RecipeBarToggle,
  checked: boolean
): PivotConfig {
  switch (toggle) {
    case "explodeMulti":
      return {
        ...config,
        multiValueStrategy: checked ? "explode" : "combined",
      };
    case "showGhostAncestors":
      return { ...config, showGhostAncestors: checked };
  }
}

/**
 * Return the current `checked` state of a RecipeBar toggle for a given
 * PivotConfig. This is the inverse of `setToggle` and is the source of
 * truth for what each checkbox in the bar should show.
 */
export function getToggleChecked(
  config: PivotConfig,
  toggle: RecipeBarToggle
): boolean {
  switch (toggle) {
    case "explodeMulti":
      return config.multiValueStrategy === "explode";
    case "showGhostAncestors":
      return config.showGhostAncestors;
  }
}

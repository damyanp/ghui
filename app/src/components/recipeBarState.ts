import type { Axis } from "$lib/bindings/Axis";
import type { Filters } from "$lib/bindings/Filters";
import type { PivotConfig } from "$lib/bindings/PivotConfig";

/** Toggles whose state lives on `PivotConfig` (persisted via view config cache). */
export type RecipeBarToggle = "explodeMulti" | "showGhostAncestors";

/** Toggles whose state lives on `Filters` (persisted via view config cache). */
export type FilterToggle = "hideClosed";

/** Toggles that are pure render concerns and live only in the frontend.
 *  These intentionally do NOT persist across app restart; they revert to
 *  the defaults defined by `RenderTogglesState`. */
export type RenderToggle = "showCounts" | "collapseSingleValue";

/** The collection of frontend-only render toggle values. */
export type RenderTogglesState = {
  showCounts: boolean;
  collapseSingleValue: boolean;
};

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
 *
 * Uses an exhaustive switch with no `default` arm so the TypeScript compiler
 * flags any new `RecipeBarToggle` variant that forgets a setter.
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

/**
 * Apply a filter-bound toggle to a `Filters` value, returning a new copy.
 *
 * Exhaustive switch — adding a new `FilterToggle` variant without handling
 * it here is a compile-time error.
 */
export function setFilterToggle(
  filters: Filters,
  toggle: FilterToggle,
  checked: boolean
): Filters {
  switch (toggle) {
    case "hideClosed":
      return { ...filters, hideClosed: checked };
  }
}

/**
 * Read the current `checked` state of a filter-bound toggle. Inverse of
 * `setFilterToggle`.
 */
export function getFilterToggleChecked(
  filters: Filters,
  toggle: FilterToggle
): boolean {
  switch (toggle) {
    case "hideClosed":
      return filters.hideClosed;
  }
}

/**
 * Apply a render-only toggle to a frontend-only state bag, returning a new
 * copy. Render toggles are not persisted; flipping them only affects how the
 * current session renders.
 *
 * Exhaustive switch — adding a new `RenderToggle` variant without handling
 * it here is a compile-time error.
 */
export function setRenderToggle(
  state: RenderTogglesState,
  toggle: RenderToggle,
  checked: boolean
): RenderTogglesState {
  switch (toggle) {
    case "showCounts":
      return { ...state, showCounts: checked };
    case "collapseSingleValue":
      return { ...state, collapseSingleValue: checked };
  }
}

/** Read the current `checked` state of a render-only toggle. */
export function getRenderToggleChecked(
  state: RenderTogglesState,
  toggle: RenderToggle
): boolean {
  switch (toggle) {
    case "showCounts":
      return state.showCounts;
    case "collapseSingleValue":
      return state.collapseSingleValue;
  }
}

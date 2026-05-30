<script lang="ts">
  import type { Axis } from "$lib/bindings/Axis";
  import type { Filters } from "$lib/bindings/Filters";
  import type { PivotConfig } from "$lib/bindings/PivotConfig";
  import { parseRecipe, recipeToString } from "$lib/recipeParser";
  import { PRESETS } from "$lib/recipePresets";
  import {
    applyText,
    getFilterToggleChecked,
    getRenderToggleChecked,
    getToggleChecked,
    setFilterToggle,
    setRenderToggle,
    setToggle,
    type FilterToggle,
    type RecipeBarToggle,
    type RenderToggle,
  } from "./recipeBarState";

  let {
    value = $bindable<PivotConfig>(),
    filters,
    showCounts = $bindable<boolean>(),
    collapseSingleValue = $bindable<boolean>(),
    onApply,
    onFiltersApply,
  }: {
    value: PivotConfig;
    filters: Filters;
    showCounts: boolean;
    collapseSingleValue: boolean;
    onApply: (cfg: PivotConfig) => void;
    onFiltersApply: (filters: Filters) => void;
  } = $props();

  // PivotConfig-backed toggles (persisted via view config cache).
  const pivotToggles: ReadonlyArray<{
    id: RecipeBarToggle;
    label: string;
  }> = [
    {
      id: "explodeMulti",
      label: "Explode multi-valued (assignees)",
    },
    {
      id: "showGhostAncestors",
      label: "Show ghost ancestors",
    },
  ];

  // Filters-backed toggles (persisted via view config cache).
  const filterToggles: ReadonlyArray<{
    id: FilterToggle;
    label: string;
  }> = [
    {
      id: "hideClosed",
      label: "Hide closed",
    },
  ];

  // Frontend-only render toggles. Intentionally NOT persisted — they revert
  // to defaults on app restart, which is acceptable for pure render concerns.
  const renderToggles: ReadonlyArray<{
    id: RenderToggle;
    label: string;
  }> = [
    { id: "showCounts", label: "Show counts" },
    { id: "collapseSingleValue", label: "Collapse single-value groups" },
  ];

  let recipeText = $state("");
  let errorText = $state<string | null>(null);

  // Tracks the recipe reference we just emitted so the sync effect can skip
  // redundant re-formatting when the update originated here.
  let lastEmittedRecipe: Axis[] | null = null;

  $effect(() => {
    const recipe = value.recipe;
    if (recipe === lastEmittedRecipe) return;
    let cancelled = false;
    recipeToString(recipe).then((text) => {
      if (!cancelled) {
        recipeText = text;
        errorText = null;
      }
    });
    return () => {
      cancelled = true;
    };
  });

  function emit(next: PivotConfig): void {
    lastEmittedRecipe = next.recipe;
    value = next;
    onApply(next);
  }

  async function applyCurrentText(): Promise<void> {
    const result = await applyText(recipeText, value, parseRecipe, recipeToString);
    if (result.ok) {
      recipeText = result.formattedText;
      emit(result.config);
    } else {
      errorText = result.error;
    }
  }

  async function pickPreset(nextRecipe: string): Promise<void> {
    if (!nextRecipe) return;
    recipeText = nextRecipe;
    const result = await applyText(nextRecipe, value, parseRecipe, recipeToString);
    if (result.ok) {
      recipeText = result.formattedText;
      emit(result.config);
    } else {
      errorText = result.error;
    }
  }

  function updateToggle(toggle: RecipeBarToggle, checked: boolean): void {
    emit(setToggle(value, toggle, checked));
  }

  function updateFilterToggle(toggle: FilterToggle, checked: boolean): void {
    onFiltersApply(setFilterToggle(filters, toggle, checked));
  }

  function updateRenderToggle(toggle: RenderToggle, checked: boolean): void {
    const next = setRenderToggle(
      { showCounts, collapseSingleValue },
      toggle,
      checked
    );
    showCounts = next.showCounts;
    collapseSingleValue = next.collapseSingleValue;
  }
</script>

<div class="flex flex-col gap-3 rounded border border-surface-300-700 bg-surface-100-900 p-3">
  <div class="flex flex-wrap items-end gap-3">
    <label class="flex min-w-[20rem] flex-1 flex-col gap-1">
      <span class="text-xs uppercase tracking-wide text-surface-700-300"
        >Recipe</span
      >
      <input
        bind:value={recipeText}
        class="rounded-lg border px-2 py-1 text-sm font-mono bg-surface-50-950 {errorText
          ? 'border-error-500'
          : 'border-surface-300-700'}"
        autocomplete="off"
        spellcheck="false"
        placeholder="e.g. Pivot(Epic) → Hierarchy"
        oninput={() => {
          errorText = null;
        }}
        onkeydown={(event) => {
          if (event.key === "Enter") {
            applyCurrentText();
          }
        }}
      />
    </label>

    <button
      class="btn preset-filled-primary-500 rounded px-3 py-1 text-sm"
      onclick={applyCurrentText}>Apply</button
    >
  </div>

  <div class="flex flex-wrap items-end gap-3">
    <label class="flex min-w-[18rem] flex-col gap-1">
      <span class="text-xs uppercase tracking-wide text-surface-700-300"
        >Preset</span
      >
      <select
        class="rounded border border-surface-300-700 bg-surface-50-950 px-2 py-1 text-sm"
        onchange={(event) => {
          pickPreset((event.currentTarget as HTMLSelectElement).value);
        }}
      >
        <option value="">— Pick a preset —</option>
        {#each PRESETS as preset}
          <option value={preset.recipe}>{preset.label} — {preset.recipe}</option>
        {/each}
      </select>
    </label>

    <div class="flex flex-wrap gap-4 pb-1">
      {#each pivotToggles as toggle}
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={getToggleChecked(value, toggle.id)}
            onchange={(event) => {
              updateToggle(toggle.id, (event.currentTarget as HTMLInputElement).checked);
            }}
          />
          <span>{toggle.label}</span>
        </label>
      {/each}
      {#each filterToggles as toggle}
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={getFilterToggleChecked(filters, toggle.id)}
            onchange={(event) => {
              updateFilterToggle(toggle.id, (event.currentTarget as HTMLInputElement).checked);
            }}
          />
          <span>{toggle.label}</span>
        </label>
      {/each}
      {#each renderToggles as toggle}
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={getRenderToggleChecked({ showCounts, collapseSingleValue }, toggle.id)}
            onchange={(event) => {
              updateRenderToggle(toggle.id, (event.currentTarget as HTMLInputElement).checked);
            }}
          />
          <span>{toggle.label}</span>
        </label>
      {/each}
    </div>
  </div>

  {#if errorText}
    <div
      class="rounded border border-error-500 bg-error-50-950 px-3 py-2 text-sm text-error-700-300"
      role="alert"
    >
      {errorText}
    </div>
  {/if}

  <details class="rounded border border-surface-300-700 bg-surface-50-950 px-3 py-2 text-sm">
    <summary class="cursor-pointer font-medium text-primary-700-300">
      Recipe grammar &amp; available fields
    </summary>

    <div class="mt-2 space-y-2">
      <p>
        A recipe is an ordered list of axes separated by <code>→</code>,
        <code>-&gt;</code>, <code>&gt;</code>, or <code>,</code>.
      </p>

      <ul class="list-disc space-y-1 pl-5">
        <li><code>Pivot(field)</code> — bucket items by their own field value.
          When <code>Hierarchy</code> is followed by exactly
          <code>Pivot(field)</code> (and nothing else), the pivot recurses,
          re-bucketing each item's sub-issues by the field at every level.</li>
        <li><code>Group(field)</code> — subgroup within the current scope.</li>
        <li><code>Hierarchy</code> — render the parent ↔ sub-issue tree.</li>
        <li><code>Sort(field)</code> — sort items by the field’s natural order.</li>
      </ul>

      <p>
        Fields: <code>Epic</code>, <code>Workstream</code>, <code>Status</code>,
        <code>Iteration</code>, <code>Kind</code>, <code>Assignee</code>,
        <code>IssueType</code>, <code>State</code>, <code>Repository</code>,
        <code>Priority</code>, <code>Blocked</code>, <code>Estimate</code>,
        <code>Type</code>. <code>IssueType</code> also accepts spaced or
        underscored forms such as <code>Issue Type</code> and
        <code>Issue_Type</code>.
      </p>
    </div>
  </details>
</div>

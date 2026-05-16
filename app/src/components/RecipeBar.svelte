<script lang="ts">
  import type { PivotConfig } from "$lib/bindings/PivotConfig";
  import { PRESETS } from "$lib/recipePresets";
  import {
    applyPreset,
    applyRecipeText,
    format,
    setToggle,
    type RecipeBarToggle,
  } from "$lib/recipeText";

  let { value = $bindable<PivotConfig>(), onApply }: {
    value: PivotConfig;
    onApply: (cfg: PivotConfig) => void;
  } = $props();

  const liveToggles: ReadonlyArray<{
    id: RecipeBarToggle;
    label: string;
    checked: (config: PivotConfig) => boolean;
  }> = [
    {
      id: "explodeMulti",
      label: "Explode multi-valued (assignees)",
      checked: (config) => config.multiValueStrategy === "explode",
    },
    {
      id: "showGhostAncestors",
      label: "Show ghost ancestors",
      checked: (config) => config.showGhostAncestors,
    },
  ];

  const shellOnlyToggles = [
    "Show counts",
    "Collapse single-valued groups",
    "Hide closed items",
  ];

  let recipeText = $state(format(value.recipe));
  let errorText = $state<string | null>(null);
  let lastAppliedRecipeText = $state(format(value.recipe));

  $effect(() => {
    const formatted = format(value.recipe);
    if (formatted !== lastAppliedRecipeText) {
      recipeText = formatted;
      lastAppliedRecipeText = formatted;
      errorText = null;
    }
  });

  function emit(next: PivotConfig): void {
    value = next;
    lastAppliedRecipeText = format(next.recipe);
    recipeText = lastAppliedRecipeText;
    errorText = null;
    onApply(next);
  }

  function applyCurrentText(): void {
    try {
      emit(applyRecipeText(value, recipeText));
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
    }
  }

  function pickPreset(nextRecipe: string): void {
    if (!nextRecipe) return;

    recipeText = nextRecipe;

    try {
      emit(applyPreset(value, nextRecipe));
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
    }
  }

  function updateToggle(toggle: RecipeBarToggle, checked: boolean): void {
    emit(setToggle(value, toggle, checked));
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
      {#each liveToggles as toggle}
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={toggle.checked(value)}
            onchange={(event) => {
              updateToggle(toggle.id, (event.currentTarget as HTMLInputElement).checked);
            }}
          />
          <span>{toggle.label}</span>
        </label>
      {/each}

      {#each shellOnlyToggles as label}
        <label
          class="flex items-center gap-2 text-sm text-surface-700-300"
          title="Blocked on additional PivotConfig bindings"
        >
          <input type="checkbox" disabled />
          <span>{label}</span>
        </label>
      {/each}
    </div>
  </div>

  {#if errorText}
    <div
      class="rounded border border-error-500 bg-error-50 px-3 py-2 text-sm text-error-700"
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
        <li><code>Pivot(field)</code> — bucket items by their own field value.</li>
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

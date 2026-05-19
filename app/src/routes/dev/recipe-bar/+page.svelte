<script lang="ts">
  import RecipeBar from "../../../components/RecipeBar.svelte";
  import type { PivotConfig } from "$lib/bindings/PivotConfig";
  import { recipeToString } from "$lib/recipeParser";

  const initialConfig: PivotConfig = {
    recipe: [{ kind: "pivot", field: "epic" }, { kind: "hierarchy" }],
    multiValueStrategy: "combined",
    showGhostAncestors: true,
  };

  let value = $state<PivotConfig>(structuredClone(initialConfig));

  let applied = $state<PivotConfig>(structuredClone(initialConfig));
  let formattedRecipe = $state("");

  $effect(() => {
    const recipe = applied.recipe;
    recipeToString(recipe).then((text) => {
      formattedRecipe = text;
    });
  });
</script>

<svelte:head>
  <title>Recipe Bar Demo</title>
</svelte:head>

<div class="mx-auto flex max-w-5xl flex-col gap-4 p-6">
  <div class="space-y-1">
    <h1 class="text-2xl font-semibold">RecipeBar demo</h1>
    <p class="text-sm text-surface-700-300">
      Standalone shell for the pivot recipe input, presets, grammar help, and
      currently available config toggles.
    </p>
  </div>

  <RecipeBar
    bind:value
    onApply={(cfg) => {
      applied = cfg;
    }}
  />

  <div class="grid gap-4 md:grid-cols-2">
    <section class="rounded border border-surface-300-700 bg-surface-100-900 p-4">
      <h2 class="mb-2 text-lg font-medium">Applied recipe</h2>
      <p class="font-mono text-sm">{formattedRecipe || "(empty — flat list)"}</p>
    </section>

    <section class="rounded border border-surface-300-700 bg-surface-100-900 p-4">
      <h2 class="mb-2 text-lg font-medium">Applied config JSON</h2>
      <pre class="overflow-x-auto text-xs">{JSON.stringify(applied, null, 2)}</pre>
    </section>
  </div>
</div>

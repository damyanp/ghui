<script lang="ts">
  import { scenarios } from "./mockups";

  let selectedId = $state(scenarios[0].id);
  const selectedScenario = $derived(
    scenarios.find((scenario) => scenario.id === selectedId) ?? scenarios[0]
  );

  const groupClass: Record<string, string> = {
    load: "bg-primary-100-900",
    edit: "bg-secondary-100-900",
    review: "bg-tertiary-100-900",
    modes: "bg-surface-200-800",
    system: "bg-surface-300-700",
  };
</script>

<div class="p-4 md:p-6 flex flex-col gap-4">
  <header class="space-y-2">
    <h1 class="h3">UI declutter mockups</h1>
    <p class="text-sm opacity-80">
      Review-only mockups from the log + telemetry plan. This route does not
      change production behavior.
    </p>
  </header>

  <section class="card p-3 space-y-3">
    <h2 class="h6">Scenario</h2>
    <div class="flex flex-wrap gap-2">
      {#each scenarios as scenario}
        <button
          class="btn variant-soft {selectedId === scenario.id
            ? 'variant-filled-primary'
            : ''}"
          onclick={() => (selectedId = scenario.id)}
        >
          {scenario.title}
        </button>
      {/each}
    </div>
    <p class="text-sm opacity-80">{selectedScenario.context}</p>
  </section>

  <section class="grid lg:grid-cols-2 gap-4">
    <article class="card p-3 space-y-3">
      <h2 class="h6">Current toolbar (reference)</h2>
      <div class="bg-surface-100-900 border border-surface-300-700 rounded p-2">
        <div class="flex flex-wrap gap-1">
          {#each selectedScenario.currentToolbar as item}
            <span
              class="chip text-xs {groupClass[item.group]} {item.active
                ? 'ring-2 ring-primary-500'
                : ''}"
            >
              {item.label}{item.count !== undefined ? ` (${item.count})` : ""}
            </span>
          {/each}
        </div>
      </div>
      <ol class="list-decimal ps-4 text-sm space-y-1">
        {#each selectedScenario.currentFlow as step}
          <li>{step}</li>
        {/each}
      </ol>
    </article>

    <article class="card p-3 space-y-3">
      <h2 class="h6">Proposed mockup</h2>
      <div class="bg-surface-100-900 border border-surface-300-700 rounded p-2">
        <div class="flex flex-wrap gap-1">
          {#each selectedScenario.proposedToolbar as item}
            <span
              class="chip text-xs {groupClass[item.group]} {item.active
                ? 'ring-2 ring-primary-500'
                : ''}"
            >
              {item.label}{item.count !== undefined ? ` (${item.count})` : ""}
            </span>
          {/each}
        </div>
      </div>
      <ol class="list-decimal ps-4 text-sm space-y-1">
        {#each selectedScenario.proposedFlow as step}
          <li>{step}</li>
        {/each}
      </ol>
    </article>
  </section>

  <section class="card p-3 space-y-2">
    <h2 class="h6">Review prompts</h2>
    <ul class="list-disc ps-4 text-sm space-y-1">
      <li>Does the proposed toolbar keep frequent actions immediately visible?</li>
      <li>Are low-frequency actions still easy to find in each scenario?</li>
      <li>
        Is the edit-to-save flow clearer than the current sequence of separate
        dialogs/panels?
      </li>
    </ul>
  </section>
</div>

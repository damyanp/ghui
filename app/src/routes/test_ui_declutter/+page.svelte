<script lang="ts">
  import { tick } from "svelte";
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import AppBarButton from "../../components/AppBarButton.svelte";
  import {
    ArrowDownToLine,
    Bubbles,
    ChartColumnBig,
    ChartGantt,
    ChevronDown,
    Ellipsis,
    GitBranch,
    KeyRound,
    ListTree,
    ReceiptText,
    RefreshCw,
    Save,
    ScrollText,
    Search,
    Undo2,
  } from "@lucide/svelte";
  import { scenarios } from "./mockups";

  let selectedId = $state(scenarios[0].id);
  const selectedScenario = $derived(
    scenarios.find((scenario) => scenario.id === selectedId) ?? scenarios[0]
  );
  let isModeMenuOpen = $state(false);
  let isActionsMenuOpen = $state(false);
  let selectedMode = $state<"items" | "xtracker" | "statistics">("items");
  let modeMenuElement = $state<HTMLDivElement | null>(null);
  let actionsMenuElement = $state<HTMLDivElement | null>(null);
  const reviewBadgeByScenario: Record<string, number> = {
    editing: 18,
    cleanup: 11,
  };
  const conflictBadgeByScenario: Record<string, number> = {
    editing: 2,
    cleanup: 7,
  };

  const currentScenarioBadge = $derived.by(() => {
    return reviewBadgeByScenario[selectedScenario.id] ?? 0;
  });

  const conflictBadge = $derived.by(() => {
    return conflictBadgeByScenario[selectedScenario.id] ?? 0;
  });

  function toggleMenu(menu: "mode" | "actions"): void {
    if (menu === "mode") {
      isModeMenuOpen = !isModeMenuOpen;
      if (isModeMenuOpen) isActionsMenuOpen = false;
      return;
    }
    isActionsMenuOpen = !isActionsMenuOpen;
    if (isActionsMenuOpen) isModeMenuOpen = false;
  }

  function selectMode(mode: "items" | "xtracker" | "statistics"): void {
    selectedMode = mode;
    isModeMenuOpen = false;
  }

  function closeMenus(): void {
    isModeMenuOpen = false;
    isActionsMenuOpen = false;
  }

  async function openAndFocusFirst(menu: "mode" | "actions"): Promise<void> {
    toggleMenu(menu);
    await tick();
    const container = menu === "mode" ? modeMenuElement : actionsMenuElement;
    const firstItem = container?.querySelector<HTMLElement>('[role="menuitem"]');
    firstItem?.focus();
  }

  function onMenuButtonKeydown(
    event: KeyboardEvent,
    menu: "mode" | "actions"
  ): void {
    if (event.key === "Escape") {
      closeMenus();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      void openAndFocusFirst(menu);
    }
  }

  function onMenuListKeydown(
    event: KeyboardEvent,
    menuElement: HTMLDivElement | null
  ): void {
    if (!menuElement) return;
    const items = Array.from(
      menuElement.querySelectorAll<HTMLElement>('[role="menuitem"]')
    );
    if (items.length === 0) return;
    const currentIndex = items.findIndex((item) => item === document.activeElement);
    if (event.key === "Escape") {
      event.preventDefault();
      closeMenus();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      const nextIndex = currentIndex < 0 ? 0 : (currentIndex + 1) % items.length;
      items[nextIndex]?.focus();
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      const nextIndex =
        currentIndex < 0 ? items.length - 1 : (currentIndex - 1 + items.length) % items.length;
      items[nextIndex]?.focus();
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      items[0]?.focus();
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      items[items.length - 1]?.focus();
    }
  }

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

  <section class="card p-3 space-y-3 relative">
    <h2 class="h6">Proposed toolbar (interactive layout)</h2>
    {#if isModeMenuOpen || isActionsMenuOpen}
      <button
        class="fixed inset-0 z-10"
        aria-label="Close menu overlays"
        onclick={closeMenus}
      ></button>
    {/if}
    <div class="rounded border border-surface-300-700">
      <AppBar padding="px-2 py-1" classes="rounded">
        {#snippet lead()}
          <AppBarButton
            icon={RefreshCw}
            text="Refresh"
            onclick={closeMenus}
          />
          <AppBarButton icon={Bubbles} text="Sanitize" onclick={closeMenus} />
          <AppBarButton
            icon={ReceiptText}
            text="Review"
            badge={currentScenarioBadge > 0 ? currentScenarioBadge : undefined}
            onclick={closeMenus}
          />
          <AppBarButton
            icon={GitBranch}
            text="Conflicts"
            badge={conflictBadge > 0 ? conflictBadge : undefined}
            onclick={closeMenus}
          />
          <AppBarButton
            icon={Save}
            text="Save"
            iconClass="bg-primary-500"
            onclick={closeMenus}
          />

          <div class="relative mx-1">
            <button
              class="btn rounded p-0.5 flex-col relative"
              aria-haspopup="menu"
              aria-expanded={isModeMenuOpen}
              onkeydown={(event) => onMenuButtonKeydown(event, "mode")}
              onclick={() => toggleMenu("mode")}
            >
              {#if selectedMode === "items"}
                <ListTree />
              {:else if selectedMode === "xtracker"}
                <ChartGantt />
              {:else}
                <ChartColumnBig />
              {/if}
              <span class="text-xs flex items-center gap-0.5">
                {#if selectedMode === "items"}Items{:else if selectedMode === "xtracker"}X-tracker{:else}Statistics{/if}
                <ChevronDown size={10} />
              </span>
            </button>
            {#if isModeMenuOpen}
              <div
                bind:this={modeMenuElement}
                role="menu"
                tabindex="-1"
                class="absolute left-0 top-12 z-20 min-w-40 rounded border border-surface-300-700 bg-surface-100-900 p-1 shadow-lg"
                onkeydown={(event) => onMenuListKeydown(event, modeMenuElement)}
              >
                <button
                  role="menuitem"
                  class="btn w-full justify-start gap-2"
                  onclick={() => selectMode("items")}
                >
                  <ListTree size={16} /> Items
                </button>
                <button
                  role="menuitem"
                  class="btn w-full justify-start gap-2"
                  onclick={() => selectMode("xtracker")}
                >
                  <ChartGantt size={16} /> X-tracker
                </button>
                <button
                  role="menuitem"
                  class="btn w-full justify-start gap-2"
                  onclick={() => selectMode("statistics")}
                >
                  <ChartColumnBig size={16} /> Statistics
                </button>
              </div>
            {/if}
          </div>

          <div class="relative mx-1">
            <button
              class="btn rounded p-0.5 flex-col relative"
              aria-haspopup="menu"
              aria-expanded={isActionsMenuOpen}
              onkeydown={(event) => onMenuButtonKeydown(event, "actions")}
              onclick={() => toggleMenu("actions")}
            >
              <Ellipsis />
              <span class="text-xs flex items-center gap-0.5">More <ChevronDown size={10} /></span>
            </button>
            {#if isActionsMenuOpen}
              <div
                bind:this={actionsMenuElement}
                role="menu"
                tabindex="-1"
                class="absolute left-0 top-12 z-20 min-w-44 rounded border border-surface-300-700 bg-surface-100-900 p-1 shadow-lg"
                onkeydown={(event) => onMenuListKeydown(event, actionsMenuElement)}
              >
                <button
                  role="menuitem"
                  class="btn w-full justify-start gap-2"
                  onclick={closeMenus}
                >
                  <Undo2 size={16} /> Undo / Redo
                </button>
                <button
                  role="menuitem"
                  class="btn w-full justify-start gap-2"
                  onclick={closeMenus}
                >
                  <Search size={16} /> Find
                </button>
              </div>
            {/if}
          </div>
        {/snippet}
        {#snippet trail()}
          <AppBarButton icon={ScrollText} text="Output" onclick={closeMenus} />
          <AppBarButton
            icon={ArrowDownToLine}
            text="Updates"
            onclick={closeMenus}
          />
          <AppBarButton icon={KeyRound} text="Pat" onclick={closeMenus} />
        {/snippet}
      </AppBar>
    </div>
    <p class="text-xs opacity-80">
      PAT stays in the right-side action group; mode and low-frequency actions
      are modeled as dropdowns with icons so interaction details are reviewable.
    </p>
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

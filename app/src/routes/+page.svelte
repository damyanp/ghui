<script lang="ts">
  import { AppBar, Switch } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import WorkItemTree from "../components/WorkItemTree.svelte";
  import RefreshButton from "../components/RefreshButton.svelte";
  import ChangesToolbarButton from "../components/ChangesToolbarButton.svelte";
  import {
    setWorkItemContext,
    WorkItemContext,
  } from "$lib/WorkItemContext.svelte";
  import SanitizeButton from "../components/SanitizeButton.svelte";
  import { ChartGantt, ListTree } from "@lucide/svelte";
  import AppBarButton from "../components/AppBarButton.svelte";
  import WorkItemExecutionTracker, {
    setWorkItemExecutionTrackerContext,
    WorkItemExecutionTrackerContext,
  } from "../components/WorkItemExecutionTracker.svelte";

  const context = setWorkItemContext(new WorkItemContext());
  setWorkItemExecutionTrackerContext(new WorkItemExecutionTrackerContext());

  async function onRefreshClicked(): Promise<void> {
    await context.refresh();
  }

  type Mode = "items" | "xtracker";
  let mode = $state<Mode>("items");

  const itemsIconClass = $derived(mode === "items" ? "bg-primary-500" : "");
  const xtrackerIconClass = $derived(
    mode === "xtracker" ? "bg-primary-500" : ""
  );
</script>

<div class="flex flex-col gap-1 h-full w-full fixed">
  <AppBar centerClasses="flex gap-1" padding="px-4 py-1">
    {#snippet lead()}
      <div
        class="content-center h-full text-lg font-black border-r rounded-2xl pe-1"
      >
        ghui
      </div>
      <RefreshButton
        progress={context.loadProgress}
        onclick={onRefreshClicked}
      />
    {/snippet}

    {#snippet children()}
      <SanitizeButton />
      <AppBarButton
        text="Items"
        icon={ListTree}
        iconClass={itemsIconClass}
        onclick={() => {
          mode = "items";
        }}
      />
      <AppBarButton
        text="X-tracker"
        icon={ChartGantt}
        iconClass={xtrackerIconClass}
        onclick={() => {
          mode = "xtracker";
        }}
      />
    {/snippet}

    {#snippet trail()}
      <ChangesToolbarButton />
      <Pat />
    {/snippet}
  </AppBar>

  {#if mode === "items"}
    <WorkItemTree />
  {:else if mode === "xtracker"}
    <WorkItemExecutionTracker />
  {:else}
    <h1>Unknown mode {mode}</h1>
  {/if}
</div>

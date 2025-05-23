<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import { Channel, invoke } from "@tauri-apps/api/core";
  import WorkItemTree from "../components/WorkItemTree.svelte";
  import RefreshButton from "../components/RefreshButton.svelte";
  import type { Data } from "$lib/bindings/Data";
  import ChangesToolbarButton from "../components/ChangesToolbarButton.svelte";
  import {
    setWorkItemContext,
    WorkItemContext,
  } from "$lib/WorkItemContext.svelte";
  import SanitizeButton from "../components/SanitizeButton.svelte";

  const context = setWorkItemContext(new WorkItemContext());

  async function onRefreshClicked(forceRefresh: boolean): Promise<void> {
    await context.refresh(forceRefresh);
  }

  onRefreshClicked(false);
</script>

<div class="grid grid-rows-[max-content_auto] gap-1 h-full w-full fixed">
  <AppBar centerClasses="flex gap-1">
    {#snippet lead()}
      <div class="content-center h-full">ghui</div>
      <RefreshButton progress={context.loadProgress} onclick={(e) => onRefreshClicked(e.shiftKey)} />
    {/snippet}

    {#snippet children()}
    <SanitizeButton />
    <ChangesToolbarButton />
    {/snippet}

    {#snippet trail()}
      <Pat />
    {/snippet}
  </AppBar>

  <WorkItemTree />
</div>

<script lang="ts">
  import { AppBar, Switch } from "@skeletonlabs/skeleton-svelte";
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
  import AppBarSwitch from "../components/AppBarSwitch.svelte";

  const context = setWorkItemContext(new WorkItemContext());

  async function onRefreshClicked(): Promise<void> {
    await context.refresh();
  }
</script>

<div class="grid grid-rows-[max-content_auto] gap-1 h-full w-full fixed">
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
    {/snippet}

    {#snippet trail()}
      <ChangesToolbarButton />
      <Pat />
    {/snippet}
  </AppBar>

  <WorkItemTree />
</div>

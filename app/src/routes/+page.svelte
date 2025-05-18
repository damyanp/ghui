<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import { Channel, invoke } from "@tauri-apps/api/core";
  import { type Data } from "../lib/data";
  import WorkItemTree from "../components/WorkItemTree.svelte";
  import RefreshButton from "../components/RefreshButton.svelte";

  let raw_data = $state<Data | undefined>(undefined);

  type Progress = number[];

  let progress = $state<number>(0);

  async function onRefreshClicked(forceRefresh: boolean): Promise<void> {
    if (progress !== 0) return;

    progress = 1;

    const getDataProgress = new Channel<Progress>();
    getDataProgress.onmessage = (message) => {
      const [retrieved, total] = message;
      if (total === 0) progress = 0;
      else progress = 1 - retrieved / total;
    };

    raw_data = await invoke<Data>("get_data", {
      forceRefresh: forceRefresh,
      progress: getDataProgress,
    });
    progress = 0;
  }

  onRefreshClicked(false);
</script>

<AppBar>
  {#snippet lead()}
    <div class="content-center h-full">ghui</div>
    <RefreshButton {progress} onclick={() => onRefreshClicked(true)} />
  {/snippet}
  {#snippet trail()}
    <Pat />
  {/snippet}
</AppBar>

{#if raw_data !== undefined}
  <WorkItemTree {raw_data} />
{/if}

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
  const getDataProgress = new Channel<Progress>();
  getDataProgress.onmessage = (message) => {
    console.log(`Message: ${JSON.stringify(message)}`);
    const [retrieved, total] = message;
    if (total === 0) progress = 0;
    else progress = 1 - retrieved / total;
  };

  async function onRefreshClicked(): Promise<void> {
    if (progress !== 0) return;

    progress = 1;
    raw_data = await invoke<Data>("get_data", { progress: getDataProgress });
    progress = 0;
  }
</script>

<AppBar>
  {#snippet lead()}
    <div class="content-center h-full">ghui</div>
    <RefreshButton {progress} onclick={onRefreshClicked} />
  {/snippet}
  {#snippet trail()}
    <Pat />
  {/snippet}
</AppBar>

{#if raw_data !== undefined}
  <WorkItemTree {raw_data} />
{/if}

<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import { Channel, invoke } from "@tauri-apps/api/core";
  import { type Data } from "../lib/data";
  import WorkItemTree from "../components/WorkItemTree.svelte";

  let raw_data = $state<Data | undefined>(undefined);

  type Progress = number[];

  let progress = $state<Progress>([0, 0]);
  const getDataProgress = new Channel<Progress>();
  getDataProgress.onmessage = (message) => {
    console.log(`Message: ${JSON.stringify(message)}`);
    progress = message;
  };

  invoke<Data>("get_data", { progress: getDataProgress }).then(
    (d) => (raw_data = d)
  );
</script>

<AppBar>
  {#snippet lead()}
    <div class="content-center h-full">ghui</div>
  {/snippet}
  {#snippet trail()}
    <Pat />
  {/snippet}
</AppBar>

{#if raw_data === undefined}
  Waiting for data... {progress[0]} / {progress[1]}
{:else}
  <WorkItemTree {raw_data} />
{/if}

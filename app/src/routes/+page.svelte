<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import { invoke } from "@tauri-apps/api/core";

  import { type Data, type WorkItem, type WorkItemId } from "../lib/data";

  const data = invoke<Data>("get_data");

  function getSubItems(item: WorkItem): string[] | null {
    if (item.data.type === "issue" && item.data.subIssues.length > 0)
      return item.data.subIssues;

    return null;
  }
</script>

<AppBar>
  {#snippet lead()}
    <div class="content-center h-full">ghui</div>
  {/snippet}
  {#snippet trail()}
    <Pat />
  {/snippet}
</AppBar>

{#await data}
  Waiting for data...
{:then result}
  Data:

  {#snippet itemList(itemIds: WorkItemId[])}
    <ul class="ps-4">
      {#each itemIds as itemId}
        {@const item = result.workItems[itemId]}
        {#if item}
          {@const subItems = getSubItems(item)}
          <li>
            {item.title}
            {#if subItems}
              {@render itemList(subItems!)}
            {/if}
          </li>
        {/if}
      {/each}
    </ul>
  {/snippet}

  {@render itemList(result.rootItems)}

  <!-- <pre>{JSON.stringify(result, null, " ")}</pre> -->
{:catch error}
  Error: {JSON.stringify(error)}
{/await}

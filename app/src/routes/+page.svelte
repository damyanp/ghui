<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import { invoke } from "@tauri-apps/api/core";

  import {
    type Data,
    type WorkItem,
    type WorkItemId,
    type Node,
  } from "../lib/data";
  import { CircleMinusIcon, CirclePlusIcon } from "@lucide/svelte";
  import { fade } from "svelte/transition";
  import { flip } from "svelte/animate";

  const data = invoke<Data>("get_data");
  let expanded = $state<string[]>([]);

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
  {#snippet expander(node: Node)}
    {#if node.children.length > 0}
      <button
        onclick={() => {
          if (expanded.includes(node.data.id)) {
            expanded = expanded.filter((i) => i !== node.data.id);
          } else {
            expanded.push(node.data.id);
          }
        }}
      >
        {#if expanded.includes(node.data.id)}
          <CircleMinusIcon size="1em" class="hover:fill-primary-500" />
        {:else}
          <CirclePlusIcon size="1em" class="hover:fill-primary-500" />
        {/if}
      </button>
    {/if}
  {/snippet}

  {#snippet itemList(nodes: Node[])}
    {#if nodes.length > 0}
      <ul class="ps-4">
        {#each nodes as node (node.data.id)}
          <li transition:fade|global animate:flip={{ duration: 500}}>
            {#if node.data.type === "group"}
              <h1 class="text-2xl border-b-2">
                <div class="relative">
                  &nbsp;
                  <div class="absolute top-0 left-0">
                    {@render expander(node)}
                  </div>
                  <div class="absolute top-0 left-8">
                    {node.data.name}
                  </div>
                </div>
              </h1>
            {/if}
            {#if node.data.type === "workItem"}
              {@const item = result.workItems[node.data.id]}
              {#if item}
                <div class="relative">
                  &nbsp;
                  <div class="absolute top-0 left-0">
                    {@render expander(node)}
                  </div>
                  <div class="absolute top-0 left-5">
                    {item.title}
                  </div>
                </div>
              {/if}
            {/if}
            {#if expanded.includes(node.data.id)}
              {@render itemList(node.children)}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}

  {@render itemList(result.rootNodes)}
  <!-- <pre>{JSON.stringify(expanded, null, " ")}</pre> -->
  <!-- <pre>{JSON.stringify(result, null, " ")}</pre> -->
{:catch error}
  Error: {JSON.stringify(error)}
{/await}

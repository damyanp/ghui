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
  import { onDestroy } from "svelte";

  let raw_data = $state<Data | undefined>(undefined);

  invoke<Data>("get_data").then((d) => (raw_data = d));

  let expanded = $state<string[]>([]);

  const data = $derived.by(() => {
    if (!raw_data) return undefined;

    let nodes = [];

    let level = 0;

    for (const node of raw_data.rootNodes) {
      if (node.level > level) continue;

      nodes.push(node);

      if (node.hasChildren && expanded.includes(node.id)) {
        level = node.level + 1;
      }
      else {
        level = node.level;
      }
    }

    return { ...raw_data, rootNodes: nodes };
  });
</script>

<AppBar>
  {#snippet lead()}
    <div class="content-center h-full">ghui</div>
  {/snippet}
  {#snippet trail()}
    <Pat />
  {/snippet}
</AppBar>

{#if data === undefined}
  Waiting for data...
{:else}
  {#snippet expander(node: Node)}
    {#if node.hasChildren}
      <button
        onclick={() => {
          if (expanded.includes(node.id)) {
            expanded = expanded.filter((i) => i !== node.id);
          } else {
            expanded.push(node.id);
          }
        }}
      >
        {#if expanded.includes(node.id)}
          <CircleMinusIcon size="1em" class="hover:fill-primary-500" />
        {:else}
          <CirclePlusIcon size="1em" class="hover:fill-primary-500" />
        {/if}
      </button>
    {/if}
  {/snippet}

  {#snippet itemList(nodes: Node[])}
    {#if nodes.length > 0}
      <ul>
        {#each nodes as node (node.id)}
          <li
            style={`padding-inline-start: ${1 * node.level}em`}
            transition:fade|global
            animate:flip={{ duration: 500 }}
          >
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
              {@const item = data.workItems[node.id]}
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
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}

  {@render itemList(data.rootNodes)}

  <!-- <pre>{JSON.stringify(expanded, null, " ")}</pre> -->
  <!-- <pre>{JSON.stringify(result, null, " ")}</pre> -->
  <!-- <pre>{JSON.stringify(raw_data?.rootNodes, null, " ")}</pre> -->
  <!-- <pre>{JSON.stringify(filteredNodes, null, " ")}</pre> -->
{/if}

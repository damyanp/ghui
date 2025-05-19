<script lang="ts">
  import { CircleMinusIcon, CirclePlusIcon } from "@lucide/svelte";
  import { fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import type { Data } from "$lib/bindings/Data";
  import type { Node } from "$lib/bindings/Node";

  let { raw_data }: { raw_data: Data } = $props();

  let expanded = $state<string[]>([]);

  const data = $derived.by(() => {
    let nodes = [];

    let level = 0;

    for (const node of raw_data.nodes) {
      if (node.level > level) continue;

      nodes.push(node);

      if (node.hasChildren && expanded.includes(node.id)) {
        level = node.level + 1;
      } else {
        level = node.level;
      }
    }

    return { ...raw_data, rootNodes: nodes };
  });
</script>

{@render itemList(data.rootNodes)}

{#snippet itemList(nodes: Node[])}
  {#if nodes.length > 0}
    <table class="w-full table-auto">
      <thead>
        <tr>
          {#each ["Title", "Status", "Iteration", "Blocked", "Kind"] as heading}
            <td>{heading}</td>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each nodes as node (node.id)}
          <tr transition:fade animate:flip={{ duration: 100 }}>
            {#if node.data.type === "group"}
              <td
                class="text-2xl border-b-2"
                style="padding-inline-start: {1 * node.level}rem"
                colspan="5"
              >
                {@render expander(node)}
                {node.data.name}
              </td>
            {:else if node.data.type === "workItem"}
              {@const item = data.workItems[node.id]}
              {#if item}
                {@const path = item.resourcePath?.split("/")}
                <td style="padding-inline-start: {1 * node.level}rem">
                  {@render expander(node)}
                  {item.title}
                  <a
                    class="underline text-blue-400"
                    target="_blank"
                    href="http://github.com{item.resourcePath}"
                    >{path?.at(-3)}#{path?.at(-1)}</a
                  >
                </td>
                <td>{item.projectItem.status?.name}</td>
                <td>{item.projectItem.iteration?.title}</td>
                <td>{item.projectItem.blocked?.name}</td>
                <td>{item.projectItem.kind?.name}</td>
              {/if}
            {/if}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/snippet}

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
  {:else}
    <div class="inline-block size-[1em]">&nbsp;</div>
  {/if}
{/snippet}

<!-- <pre>{JSON.stringify(expanded, null, " ")}</pre> -->
<!-- <pre>{JSON.stringify(result, null, " ")}</pre> -->
<!-- <pre>{JSON.stringify(raw_data?.rootNodes, null, " ")}</pre> -->
<!-- <pre>{JSON.stringify(data, null, " ")}</pre> -->

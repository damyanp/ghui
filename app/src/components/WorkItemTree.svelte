<script lang="ts">
  import {
    ChevronDown,
    ChevronRight,
  } from "@lucide/svelte";
  import { fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import type { Node } from "$lib/bindings/Node";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import WorkItemContextMenu, {
    type MenuOption,
  } from "./WorkItemContextMenu.svelte";

  let context = getWorkItemContext();

  let expanded = $state<string[]>([]);

  type ModifiedNode = Node & { modifiedDescendent: boolean };

  const data = $derived.by(() => {
    let nodes: ModifiedNode[] = [];

    let level = 0;

    for (const node of context.data.nodes) {
      if (node.level > level) {
        nodes[nodes.length - 1].modifiedDescendent =
          nodes[nodes.length - 1].modifiedDescendent || node.isModified;
        continue;
      }

      nodes.push({ ...node, modifiedDescendent: false });

      if (node.hasChildren && expanded.includes(node.id)) {
        level = node.level + 1;
      } else {
        level = node.level;
      }
    }

    return { ...context.data, rootNodes: nodes };
  });

  function contextMenu(item: WorkItem): MenuOption[] {
    let items: MenuOption[] = [];
    if (item.data.type === "issue" && item.data.trackedIssues.length > 0) {
      items.push({
        type: "action",
        title: `Convert ${item.data.trackedIssues.length} tracked issues to sub-issues`,
        action: () => convertTrackedIssuesToSubIssue(item),
      });
    }

    if (items.length === 0) return [{ type: "text", title: "No actions" }];
    else return items;
  }

  function convertTrackedIssuesToSubIssue(item: WorkItem) {
    context.convertTrackedIssuesToSubIssue(item.id);
  }
</script>

<div class="p-5 overflow-auto">
  {@render itemList(data.rootNodes)}
</div>

{#snippet itemList(nodes: ModifiedNode[])}
  {#if nodes.length > 0}
    <div
      class="grid w-full"
      style="grid-template-columns: 5fr 1fr 1fr 1fr 1fr 1fr"
    >
      {#each ["Title", "Status", "Iteration", "Blocked", "Kind", "# Tracked"] as heading}
        <div
          class="text-lg font-bold bg-surface-300-700 text-surface-contrast-300-700"
        >
          {heading}
        </div>
      {/each}
      {#each nodes as node (node.id)}
        <div
          transition:fade
          animate:flip={{ duration: 100 }}
          class={[
            "grid-cols-subgrid grid col-span-6 overflow-hidden border border-surface-200-800",
            `${node.isModified ? "bg-secondary-300-700" : node.modifiedDescendent ? "bg-secondary-50-950" : "hover:bg-surface-100-900"}`,
          ]}
          style={`padding-left: ${1 * node.level}rem;`}
        >
          {#if node.data.type === "group"}
            {@render groupRow(node)}
          {:else if node.data.type === "workItem"}
            {@render workItemRow(node)}
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet groupRow(node: ModifiedNode)}
  <div class="col-span-6 py-2 font-bold">
    {@render expander(node)}
    {node.data.type === "group" && node.data.name}
  </div>
{/snippet}

{#snippet workItemRow(node: ModifiedNode)}
  {@const item = data.workItems[node.id]}
  {#if item}
    {@const path = item.resourcePath?.split("/")}
    <div
      class="flex gap-1 py-0.5 overflow-hidden border-r border-surface-200-800 flex-nowrap"
    >
      {@render expander(node)}
      <div class="overflow-hidden whitespace-nowrap overflow-ellipsis shrink-2">
        {item.title}
      </div>
      <a
        class="text-blue-400 underline whitespace-nowrap shrink-0"
        target="_blank"
        href="http://github.com{item.resourcePath}"
      >
        {path?.at(-3)}#{path?.at(-1)}
      </a>
    </div>
    <div class="px-1 py-0.5 border-r border-surface-200-800">
      {item.projectItem.status?.name}
    </div>
    <div class="px-1 py-0.5 border-r border-surface-200-800">
      {item.projectItem.iteration?.title}
    </div>
    <div class="px-1 py-0.5 border-r border-surface-200-800">
      {item.projectItem.blocked?.name}
    </div>
    <div class="px-1 py-0.5 border-r border-surface-200-800">
      {item.projectItem.kind?.name}
    </div>
    <div class="px-1 py-0.5 border-r cursor-default border-surface-500">
      <WorkItemContextMenu items={contextMenu(item)}>
        {#snippet trigger()}
          {item.data.type === "issue" ? item.data.trackedIssues.length : ""}
        {/snippet}
      </WorkItemContextMenu>
    </div>
  {/if}
{/snippet}

{#snippet expander(node: Node)}
  {#if node.hasChildren}
    <button
      class="shrink-0"
      onclick={() => {
        if (expanded.includes(node.id)) {
          expanded = expanded.filter((i) => i !== node.id);
        } else {
          expanded.push(node.id);
        }
      }}
    >
      {#if expanded.includes(node.id)}
        <ChevronDown size="1em" class="hover:bg-primary-500" />
      {:else}
        <ChevronRight size="1em" class="hover:bg-primary-500" />
      {/if}
    </button>
  {:else}
    <div class="shrink-0 inline-block size-[1em]">&nbsp;</div>
  {/if}
{/snippet}

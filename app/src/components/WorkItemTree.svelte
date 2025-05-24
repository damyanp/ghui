<script lang="ts">
  import {
    CircleMinusIcon,
    CirclePlusIcon,
    EllipsisVertical,
    Menu,
  } from "@lucide/svelte";
  import { fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import type { Data } from "$lib/bindings/Data";
  import type { Node } from "$lib/bindings/Node";
  import * as floating from "@floating-ui/dom";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import { tick } from "svelte";
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

<div class="overflow-auto">
  {@render itemList(data.rootNodes)}
</div>

{#snippet itemList(nodes: ModifiedNode[])}
  {#if nodes.length > 0}
    <table class="w-full table-auto">
      <thead class="sticky top-0 bg-primary-50-950 outline">
        <tr>
          {#each ["Title", "Status", "Iteration", "Blocked", "Kind", "# Tracked"] as heading}
            <td>{heading}</td>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each nodes as node (node.id)}
          <tr
            transition:fade
            animate:flip={{ duration: 100 }}
            class={[
              `${node.isModified ? "bg-secondary-300-700" : node.modifiedDescendent ? "bg-secondary-50-950" : ""}`,
              "group",
            ]}
          >
            {#if node.data.type === "group"}
              {@render groupRow(node)}
            {:else if node.data.type === "workItem"}
              {@render workItemRow(node)}
            {/if}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/snippet}

{#snippet groupRow(node: ModifiedNode)}
  <td
    class="text-2xl border-b-2"
    style="padding-inline-start: {1 * node.level}rem"
    colspan="6"
  >
    {@render expander(node)}
    {node.data.type === "group" && node.data.name}
  </td>
{/snippet}

{#snippet workItemRow(node: ModifiedNode)}
  {@const item = data.workItems[node.id]}
  {#if item}
    {@const path = item.resourcePath?.split("/")}
    <td style="padding-inline-start: {1 * node.level}rem">
      {@render expander(node)}
      {item.title}
      <a
        class="text-blue-400 underline"
        target="_blank"
        href="http://github.com{item.resourcePath}"
        >{path?.at(-3)}#{path?.at(-1)}</a
      >
    </td>
    <td>{item.projectItem.status?.name}</td>
    <td>{item.projectItem.iteration?.title}</td>
    <td>{item.projectItem.blocked?.name}</td>
    <td>{item.projectItem.kind?.name}</td>
    <td class="cursor-default">
      <WorkItemContextMenu items={contextMenu(item)}>
        {#snippet trigger()}
          {item.data.type === "issue" ? item.data.trackedIssues.length : ""}
        {/snippet}
      </WorkItemContextMenu>
    </td>
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

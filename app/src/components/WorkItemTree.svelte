<script lang="ts">
  import {
    CircleMinusIcon,
    CirclePlusIcon,
  } from "@lucide/svelte";
  import { fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import type { Data } from "$lib/bindings/Data";
  import type { Node } from "$lib/bindings/Node";
  import * as floating from "@floating-ui/dom";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import { tick } from "svelte";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";

  let context = getWorkItemContext();

  let expanded = $state<string[]>([]);

  const data = $derived.by(() => {
    let nodes = [];

    let level = 0;

    for (const node of context.data.nodes) {
      if (node.level > level) continue;

      nodes.push(node);

      if (node.hasChildren && expanded.includes(node.id)) {
        level = node.level + 1;
      } else {
        level = node.level;
      }
    }

    return { ...context.data, rootNodes: nodes };
  });

  type MenuOption =
    | { type: "text"; title: string }
    | { type: "action"; title: string; action: () => void };

  let menu_options = $state<MenuOption[]>([]);

  function showMenu(e: MouseEvent, item: WorkItem) {
    if (!e.currentTarget) return;

    const menu = document.getElementById("menu") as HTMLDialogElement;
    if (!menu) return;

    const target = e.currentTarget as Element;
    if (!target) return;

    menu_options = [];

    if (item.data.type === "issue") {
      if (item.data.trackedIssues.length > 0) {
        menu_options.push({
          type: "action",
          title: `Convert ${item.data.trackedIssues.length} tracked issues to sub-issues`,
          action: () => convertTrackedIssuesToSubIssue(item),
        });
      }
    }

    if (menu_options.length === 0)
      menu_options = [{ type: "text", title: "No actions" }];

    // Use tick to ensure that the menu has figured out its layout before we
    // attempt to figure out where to position it.
    tick().then(() => {
      floating
        .computePosition(target, menu, {
          placement: "bottom-start",
          middleware: [
            floating.offset(6),
            floating.flip(),
            floating.shift({ padding: 5 }),
          ],
        })
        .then(({ x, y, placement }) => {
          Object.assign(menu.style, { left: `${x}px`, top: `${y}px` });
        });

      menu.showModal();
    });
  }

  function handleMenuSelect(option: MenuOption) {
    const menu = document.getElementById("menu") as HTMLDialogElement;
    menu.close();

    if (option.type === "action") option.action();
  }

  function convertTrackedIssuesToSubIssue(item: WorkItem) {
    context.convertTrackedIssuesToSubIssue(item.id);
  }
</script>

<div class="overflow-auto">
  {@render itemList(data.rootNodes)}
</div>

{#snippet itemList(nodes: Node[])}
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
            class={`${node.isModified ? "bg-secondary-50-950" : ""}`}
          >
            {#if node.data.type === "group"}
              <td
                class="text-2xl border-b-2"
                style="padding-inline-start: {1 * node.level}rem"
                colspan="6"
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
                <td onclick={(e) => showMenu(e, item)}
                  >{item.data.type === "issue"
                    ? item.data.trackedIssues.length
                    : ""}</td
                >
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

<dialog id="menu" closedby="any" class="absolute border p-2">
  {#each menu_options as option}
    {#if option.type === "text"}
      <button class="p-2" onclick={() => handleMenuSelect(option)}>
        {option.title}
      </button>
    {:else if option.type === "action"}
      <button
        class="btn bg-primary-500 p-2"
        onclick={() => handleMenuSelect(option)}>{option.title}</button
      >
    {/if}
  {/each}
</dialog>

<!-- <pre>{JSON.stringify(expanded, null, " ")}</pre> -->
<!-- <pre>{JSON.stringify(result, null, " ")}</pre> -->
<!-- <pre>{JSON.stringify(raw_data?.rootNodes, null, " ")}</pre> -->
<!-- <pre>{JSON.stringify(data, null, " ")}</pre> -->

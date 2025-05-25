<script lang="ts">
  import type { Node } from "$lib/bindings/Node";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { type MenuItem } from "./TreeTableContextMenu.svelte";
  import TreeTable from "./TreeTable.svelte";
  import { createRawSnippet } from "svelte";
  import type { Change } from "$lib/bindings/Change";

  let context = getWorkItemContext();

  function getContextMenuItems(node: Node): MenuItem[] {
    let items: MenuItem[] = [];

    function getDisplayName(workItem: WorkItem | undefined): string {
      if (!workItem || !workItem.resourcePath) return workItem?.title || "?";
      const path = workItem.resourcePath.split("/");
      return `${path.at(-3) ?? "?"}#${path.at(-1) ?? "?"}`;
    }

    function describeChange(change: Change): string {
      switch (change.data.type) {
        case "setParent": {
          let parent = context.data.workItems[change.data.value];
          let parentDisplay = getDisplayName(parent) || "???";
          return `Set parent to '${parentDisplay}'`;
        }
        case "addToProject": {
          return "Add to project";
        }
        default: {
          // All other types just set a field to a value
          return `Set ${change.data.type} to '${change.data.value}'`;
        }
      }
    }

    if (node.data.type === "workItem") {
      let item = context.data.workItems[node.id];
      if (item) {
        if (item.data.type === "issue" && item.data.trackedIssues.length > 0) {
          items.push({
            type: "action",
            title: `Convert ${item.data.trackedIssues.length} tracked issues to sub-issues`,
            action: () => convertTrackedIssuesToSubIssue(item),
          });
        }
        if (context.data.changes.data) {
          let changes = Object.values(context.data.changes.data).filter(
            (i) => i?.workItemId === item.id
          ) as Change[];

          for (const change of changes) {
            items.push({
              type: "action",
              title: `Revert change: ${describeChange(change)}`,
              action: () => context.removeChange(change),
            });
          }
        }
      }
    }
    if (items.length === 0) return [{ type: "text", title: "No actions" }];
    else return items;
  }

  function convertTrackedIssuesToSubIssue(item: WorkItem) {
    context.convertTrackedIssuesToSubIssue(item.id);
  }

  let rows = $derived.by(() => {
    return context.data.nodes.map((n) => {
      return { ...n, isGroup: n.data.type === "group" };
    });
  });

  let columns = [
    { name: "Title", width: "5fr", render: renderTitle },
    {
      name: "Type",
      width: "1fr",
      render: renderTextCell((i) => {
        if (i.data.type === "issue") return i.data.issueType;
        return null;
      }),
    },
    {
      name: "Updated",
      width: "1fr",
      render: renderTextCell((i) => {
        const projectUpdate = i.projectItem.updatedAt;
        const itemUpdate = i.updatedAt ?? projectUpdate;

        return itemUpdate < projectUpdate ? projectUpdate : itemUpdate;
      }),
    },
    {
      name: "State",
      width: "1fr",
      render: renderTextCell((i) => {
        if (i.data.type === "issue") return i.data.state.toString();
        else if (i.data.type === "pullRequest") return i.data.state.toString();
        else return null;
      }),
    },
    {
      name: "Status",
      width: "1fr",
      render: renderTextCell((i) => i.projectItem.status?.name ?? null),
    },
    {
      name: "Iteration",
      width: "1fr",
      render: renderTextCell((i) => i.projectItem.iteration?.title ?? null),
    },
    {
      name: "Blocked",
      width: "1fr",
      render: renderTextCell((i) => i.projectItem.blocked?.name ?? null),
    },
    {
      name: "Kind",
      width: "1fr",
      render: renderTextCell((i) => i.projectItem.kind?.name ?? null),
    },
    {
      name: "Epic",
      width: "1fr",
      render: renderTextCell((i) => i.projectItem.epic?.name ?? null),
    },
    {
      name: "#Tracked",
      width: "1fr",
      render: renderTextCell((i) => {
        if (i.data.type === "issue" && i.data.trackedIssues.length > 0)
          return i.data.trackedIssues.length.toString();
        else return null;
      }),
    },
  ];

  function getGroup(n: Node) {
    if (n.data.type === "group") return n.data.name;
  }

  function getItem(n: Node) {
    return context.data.workItems[n.id];
  }

  function renderTextCell(getText: (item: WorkItem) => string | null) {
    return createRawSnippet((itemGetter: () => WorkItem | undefined) => {
      return {
        render: () => {
          const item = itemGetter();
          return `<span>${(item && getText(item)) || "&nbsp;"}</span>`;
        },
      };
    });
  }

  async function onRowDragDrop(draggedRowId: string, droppedOntoRowId: string) {
    console.log(`Item ${draggedRowId} dropped onto ${droppedOntoRowId}`);

    let targetNode = rows.find((row) => row.id === droppedOntoRowId);
    if (!targetNode) {
      console.log(
        `WARNING: couldn't find target node with id ${droppedOntoRowId}`
      );
      return;
    }

    let draggedNode = rows.find((row) => row.id === draggedRowId);
    if (!draggedNode) {
      console.log(
        `WARNING: couldn't find dragged node with id ${draggedRowId}`
      );
      return;
    }

    let change: Change | undefined;

    if (draggedNode.data.type === "workItem") {
      if (targetNode.data.type === "group") {
        // Currently the only group is epic
        change = {
          workItemId: draggedRowId,
          data: {
            type: "epic",
            value: targetNode.data.name,
          },
        };
      } else {
        change = {
          workItemId: draggedRowId,
          data: {
            type: "setParent",
            value: droppedOntoRowId,
          },
        };
      }
    }

    console.log(`Change: ${JSON.stringify(change)}`);

    if (change) await context.addChange(change);
  }
</script>

<TreeTable
  {rows}
  {columns}
  {getGroup}
  {getItem}
  {renderGroup}
  {getContextMenuItems}
  {onRowDragDrop}
/>

{#snippet renderGroup(name: string | undefined)}
  {name}
{/snippet}

{#snippet renderTitle(item: WorkItem | undefined)}
  {#if item}
    {@const path = item.resourcePath?.split("/")}
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
  {:else}
    &nbsp;
  {/if}
{/snippet}

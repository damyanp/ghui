<script lang="ts">
  import type { Node } from "$lib/bindings/Node";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { type MenuItem } from "./TreeTableContextMenu.svelte";
  import TreeTable from "./TreeTable.svelte";
  import { createRawSnippet, type Snippet } from "svelte";
  import type { Change } from "$lib/bindings/Change";
  import type { DelayLoad } from "$lib/bindings/DelayLoad";
  import { type FieldOptionId } from "$lib/bindings/FieldOptionId";
  import { type ProjectItem } from "$lib/bindings/ProjectItem";
  import { type IssueState } from "$lib/bindings/IssueState";
  import { type PullRequestState } from "$lib/bindings/PullRequestState";
  import { type Fields } from "$lib/bindings/Fields";
  import ItemMiniIcon from "./ItemMiniIcon.svelte";

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
        case "issueType": {
          let item = context.data.workItems[change.workItemId];
          if (item?.data.type === "issue") {
            if (item.data.issueType.loadState === "loaded")
              if (item.data.issueType.value)
                return `Set issue type to ${item.data.issueType.value}`;
              else return "Clear issue type";
            else return "Error: issue type not loaded";
          } else return "Error: issue type change for non-issue!";
        }
        default: {
          // All other types just set a field to a value
          return `Set ${change.data.type} to '${context.getFieldOption(change.data.type as keyof Fields, change.data.value)}'`;
        }
      }
    }

    if (node.data.type === "workItem") {
      let item = context.data.workItems[node.id];
      if (item) {
        items.push({
          type: "action",
          title: "Refresh",
          action: () => context.itemUpdateBatcher.add(node.id, true),
        });

        if (
          item.data.type === "issue" &&
          item.data.trackedIssues.loadState === "loaded" &&
          item.data.trackedIssues.value.length > 0
        ) {
          items.push({
            type: "action",
            title: `Convert ${item.data.trackedIssues.value.length} tracked issues to sub-issues`,
            action: () => context.convertTrackedIssuesToSubIssue(item.id),
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

  let rows = $derived.by(() => {
    return context.data.nodes.map((n) => {
      return { ...n, isGroup: n.data.type === "group" };
    });
  });

  let columns = $state([
    { name: "Title", width: "5fr", render: renderTitle },
    {
      name: "Type",
      width: "1fr",
      render: renderType,
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
      render: renderState,
    },
    {
      name: "Status",
      width: "1fr",
      render: renderStatus,
    },
    {
      name: "Iteration",
      width: "1fr",
      render: renderIteration,
    },
    {
      name: "Blocked",
      width: "1fr",
      render: renderBlocked,
    },
    {
      name: "Kind",
      width: "1fr",
      render: renderKind,
    },
    {
      name: "Epic",
      width: "1fr",
      render: renderEpic,
    },
    {
      name: "Assigned To",
      width: "2fr",
      render: renderTextCell((i) => {
        if (i.data.type === "issue" || i.data.type === "pullRequest") {
          return i.data.assignees && i.data.assignees.length > 0
            ? i.data.assignees.join(", ")
            : "";
        }
        return "";
      }),
    },
    {
      name: "#Tracked",
      width: "1fr",
      render: renderTextCell((i) => {
        if (
          i.data.type === "issue" &&
          i.data.trackedIssues.loadState === "loaded" &&
          i.data.trackedIssues.value.length > 0
        )
          return i.data.trackedIssues.value.length.toString();
        else return null;
      }),
    },
  ]);

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

    if (change) await context.addChange(change);
  }

  function onRowFirstVisible(row: Node) {
    if (row.data.type === "workItem") context.updateWorkItem(row.id);
  }

  let expanded = $state([]);
</script>

<TreeTable
  {rows}
  bind:columns
  bind:expanded
  {getGroup}
  {getItem}
  {renderGroup}
  {getContextMenuItems}
  {onRowDragDrop}
  {onRowFirstVisible}
/>

{#snippet renderGroup(name: string | undefined)}
  {name}
{/snippet}

{#snippet renderTitle(item: WorkItem | undefined)}
  {#if item}
    {@const path = item.resourcePath?.split("/")}
    <div class="overflow-hidden whitespace-nowrap overflow-ellipsis shrink-2">
      <ItemMiniIcon workItemData={item.data} />
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

{#snippet renderDelayLoad<T>(item: DelayLoad<T>, snippet: Snippet<[T]>)}
  {#if item.loadState === "notLoaded"}
    <div class="flex items-center justify-center h-full w-full">
      <div class="bg-surface-300-700 rounded w-3/4 h-3/4"></div>
    </div>
  {:else}
    {@render snippet(item.value)}
  {/if}
{/snippet}

{#snippet renderType(item: WorkItem)}
  {#if item.data.type === "issue"}
    {#snippet render(issueType: string | null)}
      {#if issueType}
        {issueType}
      {:else}
        &nbsp;
      {/if}
    {/snippet}
    {@render renderDelayLoad(item.data.issueType, render)}
  {:else}
    &nbsp;
  {/if}
{/snippet}

{#snippet renderState(item: WorkItem)}
  {#if item.data.type === "issue" || item.data.type === "pullRequest"}
    {#snippet render(state: IssueState | PullRequestState)}
      {state.toString()}
    {/snippet}
    {@render renderDelayLoad(item.data.state, render)}
  {:else}
    &nbsp;
  {/if}
{/snippet}

{#snippet renderStatus(item: WorkItem)}
  {@render renderCustomField(item, "status")}
{/snippet}
{#snippet renderIteration(item: WorkItem)}
  {@render renderDelayLoadCustomField(item, "iteration")}
{/snippet}
{#snippet renderBlocked(item: WorkItem)}
  {@render renderDelayLoadCustomField(item, "blocked")}
{/snippet}
{#snippet renderKind(item: WorkItem)}
  {@render renderDelayLoadCustomField(item, "kind")}
{/snippet}
{#snippet renderEpic(item: WorkItem)}
  {@render renderCustomField(item, "epic")}
{/snippet}

{#snippet renderDelayLoadCustomField(item: WorkItem, field: keyof ProjectItem)}
  {#snippet render(value: FieldOptionId | null)}
    {context.getFieldOption(field as keyof Fields, value)}
  {/snippet}
  {@render renderDelayLoad(
    item.projectItem[field] as DelayLoad<FieldOptionId | null>,
    render
  )}
{/snippet}

{#snippet renderCustomField(item: WorkItem, field: keyof ProjectItem)}
  {context.getFieldOption(
    field as keyof Fields,
    item.projectItem[field] as FieldOptionId
  )}
{/snippet}

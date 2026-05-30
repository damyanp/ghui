<script lang="ts">
  import dayjs from "dayjs";
  import isBetween from "dayjs/plugin/isBetween";
  import type { Node } from "$lib/bindings/Node";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import {
    getWorkItemContext,
    directLinkHRef,
    projectLinkHRef,
    linkTitle,
  } from "$lib/WorkItemContext.svelte";
  import { type MenuItem } from "./TreeTableContextMenu.svelte";
  import {
    computeGroupChildCounts,
    computeVisibleRows,
  } from "./workItemTreeRows";
  import TreeTable, { type Column } from "./TreeTable.svelte";
  import { createRawSnippet, onMount, type Snippet } from "svelte";
  import type { Change } from "$lib/bindings/Change";
  import type { DelayLoad } from "$lib/bindings/DelayLoad";
  import { type FieldOptionId } from "$lib/bindings/FieldOptionId";
  import { type ProjectItem } from "$lib/bindings/ProjectItem";
  import { type IssueState } from "$lib/bindings/IssueState";
  import { type PullRequestState } from "$lib/bindings/PullRequestState";
  import { type Fields } from "$lib/bindings/Fields";
  import type { Filters } from "$lib/bindings/Filters";
  import type { FilterableField } from "$lib/filterableFields";
  import type { WorkItemId } from "$lib/bindings/WorkItemId";
  import ItemMiniIcon from "./ItemMiniIcon.svelte";
  import TableFieldSelect from "./TableFieldSelect.svelte";
  import { type FieldOption } from "$lib/bindings/FieldOption";
  import { type Iteration } from "$lib/bindings/Iteration";
  import { SvelteSet } from "svelte/reactivity";
  import * as octicons from "@primer/octicons";
  import SingleSelectColumnMenu from "./SingleSelectColumnMenu.svelte";
  import IterationColumnMenu from "./IterationColumnMenu.svelte";
  import AddItemDialog from "./AddItemDialog.svelte";
  import { ghostContextMenuItems } from "$lib/ghostRouting";

  dayjs.extend(isBetween);

  let context = getWorkItemContext();

  // State for the "Add item from URL" dialog used within WorkItemTree (for
  // example, from its context menu). The main toolbar in +page.svelte uses a
  // separate AddItemDialog instance and state.
  let addItemDialogOpen = $state(false);
  let addItemParentId = $state<WorkItemId | undefined>(undefined);
  let addItemEpicId = $state<FieldOptionId | null | undefined>(undefined);

  function openAddItemDialog(
    parentId?: WorkItemId,
    epicId?: FieldOptionId | null
  ) {
    addItemParentId = parentId;
    addItemEpicId = epicId;
    // Workaround for a bits-ui (ContextMenu) <-> @zag-js/dialog (Modal) interaction:
    // when this runs from a ContextMenu item's onSelect, bits-ui has just set
    // `body.style.pointerEvents = "none"` and won't restore it for ~24ms. The
    // Skeleton Modal is built on zag-js's dialog, which on open captures the
    // current `body.style.pointerEvents` as the "original" value to restore on
    // close. If it captures "none", closing the dialog leaves the body frozen
    // (https://github.com/huntabyte/bits-ui/issues/1639 covers a similar race).
    // Clear it explicitly so zag captures an empty string.
    if (typeof document !== "undefined") {
      document.body.style.pointerEvents = "";
    }
    addItemDialogOpen = true;
  }

  // Scrolls the row with the given id into the centre of the viewport, used
  // by the ghost-row context menu's "Jump to primary occurrence" action. If
  // the row is not currently in the DOM (e.g. an ancestor is collapsed) the
  // call is a silent no-op aside from a debug log.
  function jumpToRowById(id: string) {
    if (typeof document === "undefined") return;
    const element = document.querySelector<HTMLElement>(
      `[data-row-id="${CSS.escape(id)}"]`
    );
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "center" });
    } else {
      console.debug(
        `[WorkItemTree] jumpToRowById: row ${id} not currently in DOM`
      );
    }
  }

  function getContextMenuItems(
    node: Node,
    column?: Column<WorkItem>
  ): MenuItem[] {
    // Ghost rows are reflections of a primary occurrence; they don't support
    // edits, refresh, filtering, or revert. Show only a "Jump to primary"
    // action (or a text fallback when no primary is in view) so the menu is
    // never empty and edit affordances stay hidden.
    //
    // Ghost rows are always work-item rows (groups are never ghosts), so
    // `node.data` narrows to the `workItem` variant here. We pass the
    // underlying work-item id — not `node.id`, which is render-position
    // unique and won't match other rows for the same work item.
    if (node.isGhost) {
      if (node.data.type !== "workItem") return [{ type: "text", title: "No actions" }];
      return ghostContextMenuItems(rows, node.data.workItemId, jumpToRowById);
    }

    let items: MenuItem[] = [];

    function getFilterableField(
      column: Column<WorkItem> | undefined
    ): FilterableField | undefined {
      if (column && context.isFilterableField(column.name)) {
        return column.name;
      }
      return undefined;
    }

    function quickFilterItems(
      field: FilterableField,
      item: WorkItem
    ): MenuItem[] {
      const value = context.getFilterableFieldValue(field, item);
      // Skip quick-filter actions while the underlying value is still loading;
      // otherwise we'd treat an unloaded value as `(none)` and apply the wrong
      // filter.
      if (value === undefined) return [];
      const label = value
        ? (context.getFieldOption(field, value) ?? "(unknown)")
        : "(none)";
      const allOptionIds = context.getFilterableFieldOptionIds(field);
      const currentFilter = context.getFilter(field);
      const filterItems: MenuItem[] = [
        {
          type: "action",
          title: `Show only ${field} = "${label}"`,
          action: () =>
            context.setFilter(
              field,
              allOptionIds.filter((id) => id !== value)
            ),
        },
        {
          type: "action",
          title: `Hide all ${field} = "${label}"`,
          action: () => {
            const next = new Set(currentFilter);
            next.add(value);
            context.setFilter(field, Array.from(next));
          },
        },
      ];
      if (currentFilter.length > 0) {
        filterItems.push({
          type: "action",
          title: `Clear ${field} filter`,
          action: () => context.setFilter(field, []),
        });
      }
      filterItems.push({ type: "separator" });
      return filterItems;
    }

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
      const workItemId = node.data.workItemId;
      let item = context.data.workItems[workItemId];
      if (item) {
        const filterField = getFilterableField(column);
        if (filterField) {
          items.push(...quickFilterItems(filterField, item));
        }

        items.push({
          type: "action",
          title: "Refresh",
          action: () => context.itemUpdateBatcher.add(workItemId, true),
        });

        if (item.data.type === "issue") {
          items.push({
            type: "action",
            title: "Add child issue from URL…",
            action: () => openAddItemDialog(item.id),
          });
        }

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
    } else if (node.data.type === "group") {
      const groupData = node.data;
      items.push({
        type: "action",
        title: "Add issue to this group from URL…",
        action: () => openAddItemDialog(undefined, groupData.fieldOptionId),
      });
    }
    if (items.length === 0) return [{ type: "text", title: "No actions" }];
    else return items;
  }

  // Count of workItem descendants for each group node. Used by the showCounts
  // decorator and the collapseSingleValue toggle (see workItemTreeRows.ts).
  let groupChildCounts = $derived(computeGroupChildCounts(context.data.nodes));

  // When collapseSingleValue is on, hide group rows that are redundant: buckets
  // holding a single work item, or buckets that are the only distinct value
  // among their siblings. The items then render inline at their own level.
  let rows = $derived(
    computeVisibleRows(context.data.nodes, context.collapseSingleValue)
  );

  let columns = $state<Column<WorkItem>[]>([
    { name: "Title", width: "5fr", disableMenu: true, render: renderTitle },
    singleSelectColumn("kind", renderKind),
    singleSelectColumn("workstream", renderWorkstream),
    singleSelectColumn("status", renderStatus),
    {
      name: "iteration",
      width: "1fr",
      render: renderIteration,
      getMenuIconSVG: getCustomFieldColumnMenuSVG,
      renderMenuContent: renderIterationFieldMenuContent,
    },
    singleSelectColumn("blocked", renderBlocked),
    singleSelectColumn("epic", renderEpic),
    singleSelectColumn("estimate", renderEstimate),
    singleSelectColumn("priority", renderPriority),
    {
      name: "Assigned",
      width: "1fr",
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
  ]);

  let hiddenColumns = $state<Column<WorkItem>[]>([]);

  // Load (on mount) and save (on unmount) column state
  onMount(() => {
    type ColState = {
      name: string;
      width: string;
      visible: boolean;
    };

    const columnDefs = new Map(
      columns.map((c) => {
        return [c.name, c];
      })
    );

    let state = JSON.parse(
      localStorage.getItem("work-item-tree-columns") || "[]"
    ) as ColState[];
    if (!state) state = [];

    columns = [];
    hiddenColumns = [];

    state.forEach((columnState) => {
      let column = columnDefs.get(columnState.name);
      if (!column) {
        console.log(
          `Have stored state for unknown column '${columnState.name}'`
        );
        return;
      }

      column.width = columnState.width;

      if (columnState.visible) columns.push(column);
      else hiddenColumns.push(column);

      columnDefs.delete(columnState.name);
    });

    // Any remaining columns are assumed to be visible
    columnDefs.values().forEach((c) => columns.push(c));

    return () => {
      let state: ColState[] = [];

      columns.forEach((c) => {
        state.push({ name: c.name, width: c.width, visible: true });
      });

      hiddenColumns.forEach((c) => {
        state.push({ name: c.name, width: c.width, visible: false });
      });

      localStorage.setItem("work-item-tree-columns", JSON.stringify(state));
    };
  });

  function singleSelectColumn(
    name: string,
    render: Snippet<[WorkItem]>
  ): Column<WorkItem> {
    return {
      name,
      width: "1fr",
      render,
      getMenuIconSVG: getCustomFieldColumnMenuSVG,
      renderMenuContent: renderSingleSelectFieldMenuContent,
    };
  }

  function getCustomFieldColumnMenuSVG(column: Column<WorkItem>) {
    const fieldName = column.name as keyof Fields;
    if (context.getFilter(fieldName).length > 0)
      return octicons["filter"].toSVG();
    return undefined;
  }

  function getGroup(n: Node) {
    if (n.data.type === "group") {
      return { name: n.data.name, count: groupChildCounts.get(n.id) ?? 0 };
    }
  }

  function getItem(n: Node) {
    if (n.data.type !== "workItem") return undefined;
    return context.data.workItems[n.data.workItemId];
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

    // Both `draggedRowId` and `droppedOntoRowId` are render-position keys
    // (path-prefixed Node ids), not WorkItemIds. Resolve the underlying
    // work-item ids from the nodes' `data` payload before recording the
    // change — Change.workItemId / setParent.value must reference the
    // real WorkItemId, not the render key.
    if (draggedNode.data.type === "workItem") {
      const draggedWorkItemId = draggedNode.data.workItemId;
      if (targetNode.data.type === "group") {
        // Currently the only group is epic
        change = {
          workItemId: draggedWorkItemId,
          data: {
            type: "epic",
            value: targetNode.data.fieldOptionId,
          },
        };
      } else if (targetNode.data.type === "workItem") {
        change = {
          workItemId: draggedWorkItemId,
          data: {
            type: "setParent",
            value: targetNode.data.workItemId,
          },
        };
      }
    }

    if (change) await context.addChange(change);
  }

  function onRowFirstVisible(row: Node) {
    if (row.data.type === "workItem") context.updateWorkItem(row.data.workItemId);
  }

  function isCurrentIteration(option: FieldOption<Iteration>) {
    const start = dayjs(option.data.startDate);
    const end = start.add(Number(option.data.duration) - 1, "days");
    const now = dayjs();

    return now.isBetween(start, end);
  }

  let expanded = $state(context.workItemTreeExpandedItems);
  let visibleRows = $state(new SvelteSet<string>());

  function matchesFilter(row: Node, filterText: string) {
    const item = getItem(row);
    if (!item) return true;

    return item.title.toLowerCase().includes(filterText.toLowerCase());
  }
</script>

<TreeTable
  {rows}
  bind:columns
  bind:hiddenColumns
  bind:expanded
  bind:visibleRows
  {getGroup}
  {getItem}
  {renderGroup}
  {getContextMenuItems}
  {onRowDragDrop}
  {onRowFirstVisible}
  {matchesFilter}
/>

<AddItemDialog
  bind:open={addItemDialogOpen}
  parentId={addItemParentId}
  epicId={addItemEpicId}
/>

{#snippet renderSingleSelectFieldMenuContent(column: Column<WorkItem>)}
  {@const fieldName = column.name as keyof Fields}
  <SingleSelectColumnMenu
    field={context.getSingleSelectField(fieldName)}
    filter={context.getFilter(fieldName)}
    onFilterChange={(filter) => context.setFilter(fieldName, filter)}
  />
{/snippet}

{#snippet renderIterationFieldMenuContent(column: Column<WorkItem>)}
  <IterationColumnMenu fieldName={column.name as keyof Fields} />
{/snippet}

{#snippet renderGroup(group: { name: string; count: number } | undefined)}
  {#if group}
    {group.name}{#if context.showCounts}
      <span class="ml-2 text-surface-700-300">({group.count})</span>
    {/if}
  {/if}
{/snippet}

{#snippet renderTitle(item: WorkItem | undefined)}
  {#if item}
    <div class="overflow-hidden whitespace-nowrap overflow-ellipsis shrink-2">
      <ItemMiniIcon workItemData={item.data} />
      {item.title}
    </div>
    <a
      class="text-blue-400 underline whitespace-nowrap shrink-0"
      target="_blank"
      href={directLinkHRef(item)}
    >
      {linkTitle(item)}
    </a>
    {#if projectLinkHRef(item)}
      <a
        class="text-blue-400 whitespace-nowrap shrink-0 ml-1"
        target="_blank"
        href={projectLinkHRef(item)}
        title="View in project"
      >
        {@html octicons["table"].toSVG({ width: 14 })}
      </a>
    {/if}
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
  {@render renderLoadedCustomField(item, "status")}
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
{#snippet renderWorkstream(item: WorkItem)}
  {@render renderDelayLoadCustomField(item, "workstream")}
{/snippet}
{#snippet renderEpic(item: WorkItem)}
  {@render renderLoadedCustomField(item, "epic")}
{/snippet}
{#snippet renderEstimate(item: WorkItem)}
  {@render renderLoadedCustomField(item, "estimate")}
{/snippet}
{#snippet renderPriority(item: WorkItem)}
  {@render renderLoadedCustomField(item, "priority")}
{/snippet}

{#snippet renderDelayLoadCustomField(item: WorkItem, field: keyof Fields)}
  {#snippet render(value: FieldOptionId | undefined)}
    {@render renderCustomField(item, field, value)}
  {/snippet}
  {#key item.projectItem[field as keyof ProjectItem]}
    {@render renderDelayLoad(
      item.projectItem[field as keyof ProjectItem] as DelayLoad<
        FieldOptionId | undefined
      >,
      render
    )}
  {/key}
{/snippet}

{#snippet renderLoadedCustomField(item: WorkItem, field: keyof Fields)}
  {#key item.projectItem[field as keyof ProjectItem]}
    {@render renderCustomField(
      item,
      field,
      item.projectItem[field as keyof ProjectItem] as FieldOptionId | undefined
    )}
  {/key}
{/snippet}

{#snippet renderCustomField(
  item: WorkItem,
  field: keyof Fields,
  value: FieldOptionId | undefined
)}
  {#if field === "iteration"}
    <TableFieldSelect
      field={context.getIterationField(field)}
      defaultValue={value}
      isGoodDefault={isCurrentIteration}
      onValueChange={(newValue) => context.setFieldValue(item, field, newValue)}
    >
      {#snippet renderOption(option)}
        {#if option}
          <div
            class="flex items-center justify-items-center {option &&
              isCurrentIteration(option) &&
              'bg-secondary-500'}"
          >
            <div class="flex-1 pr-2">{option.value}</div>
            <div class="flex-1 align-center text-xs">
              {dayjs(option.data.startDate).format("MM-DD")}
              -
              {dayjs(option.data.startDate)
                .add(Number(option.data.duration) - 1, "days")
                .format("MM-DD")}
            </div>
          </div>
        {:else}
          -
        {/if}
      {/snippet}
    </TableFieldSelect>
  {:else}
    <TableFieldSelect
      field={context.getSingleSelectField(field)}
      defaultValue={value}
      onValueChange={(newValue) => context.setFieldValue(item, field, newValue)}
    />
  {/if}
{/snippet}

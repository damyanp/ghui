<script lang="ts" module>
  import type { GhostAwareNodeData } from "$lib/ghostRouting";

  export type Column<ITEM> = {
    name: string;
    width: string;
    render: Snippet<[ITEM]>;
    disableMenu?: boolean;
    getMenuIconSVG?: (column: Column<ITEM>) => string | undefined;
    renderMenuContent?: Snippet<[Column<ITEM>]>;
  };

  // `data` carries a discriminated union so ghost-routing code can look up
  // a row's underlying work-item id (only `Node.id` — the render-position
  // unique key — is exposed here). `T` must extend the minimal shape so
  // `handleRowClick` can narrow on `row.data.type` without unsafe casts.
  type Row<T extends GhostAwareNodeData> = {
    level: number;
    id: string;
    hasChildren: boolean;
    isModified: boolean;
    isGhost: boolean;
    isGroup: boolean;
    data: T;
  };

  type Props<T extends GhostAwareNodeData, GROUP, ITEM> = {
    rows: Row<T>[];
    columns: Column<ITEM>[];
    hiddenColumns: Column<ITEM>[];

    // These exist outside the TreeTable itself so they get persisted even if
    // the table unmounted.
    expanded?: string[];
    visibleRows?: SvelteSet<string>;

    getGroup: (row: Row<T>) => GROUP;
    getItem: (row: Row<T>) => ITEM | undefined;
    renderGroup: Snippet<[GROUP]>;
    getContextMenuItems: (row: Row<T>, column?: Column<ITEM>) => MenuItem[];
    onRowDragDrop?: (draggedRowId: string, droppedOntoRowId: string) => void;
    onRowFirstVisible?: (row: Row<T>) => void;
    matchesFilter?: (row: Row<T>, filter: string) => boolean;
  };
</script>

<script lang="ts" generics="T extends GhostAwareNodeData, GROUP, ITEM">
  import { ChevronDown, ChevronRight, CirclePlus } from "@lucide/svelte";
  import { tick, type Snippet } from "svelte";
  import TreeTableContextMenu, {
    type MenuItem,
  } from "./TreeTableContextMenu.svelte";
  import { onFirstVisible } from "$lib/OnVirstVisible";
  import { SvelteSet } from "svelte/reactivity";
  import type { Attachment } from "svelte/attachments";
  import FindDialog from "./FindDialog.svelte";
  import TableColumnMenu from "./TableColumnMenu.svelte";
  import AddColumnButton from "./AddColumnButton.svelte";
  import { recordTelemetry } from "$lib/WorkItemContext.svelte";
  import { isRowDraggable, findPrimaryRow } from "$lib/ghostRouting";

  let {
    columns = $bindable(),
    hiddenColumns = $bindable(),
    expanded = $bindable([]),
    visibleRows = $bindable(new SvelteSet<string>()),
    ...props
  }: Props<T, GROUP, ITEM> = $props();

  type MRow<U extends GhostAwareNodeData> = Row<U> & { modifiedDescendent: boolean };

  let filterText = $state<string>("");

  const rows: MRow<T>[] = $derived.by(() => {
    let rows: MRow<T>[] = [];

    if (filterText.trim().length > 1 && props.matchesFilter) {
      for (const row of props.rows) {
        if (props.matchesFilter(row, filterText))
          rows.push({ ...row, modifiedDescendent: false });
      }
    } else {
      let level = 0;

      for (const row of props.rows) {
        // Skip unexpanded rows, but figure out if any of the ones that were
        // skipped were modified.
        if (row.level > level) {
          rows[rows.length - 1].modifiedDescendent =
            rows[rows.length - 1].modifiedDescendent || row.isModified;
          continue;
        }

        rows.push({ ...row, modifiedDescendent: false });

        if (row.hasChildren && expanded.includes(row.id)) {
          level = row.level + 1;
        } else {
          level = row.level;
        }
      }
    }
    return [...rows];
  });

  const gridTemplateColumns = $derived(columns.map((c) => c.width).join(" "));

  $effect(() => {
    tick().then(() => {
      for (let index = 0; index < columns.length; index++) {
        const element = document.getElementById(`column-index-${index}`);
        if (!element) {
          console.log(`Couldn't find column ${index}`);
          continue;
        }

        columns[index].width = `${element.getBoundingClientRect().width}px`;
      }
    });
  });

  const gridColumn = $derived(
    `span ${columns.length} / span ${columns.length};`
  );

  let draggedRowId: string | null = $state(null);
  let currentDropRowId: string | null = $state(null);

  // Tracks which column was right-clicked, so getContextMenuItems can offer
  // cell-aware actions (eg. quick filters). Reset on each contextmenu event
  // via the capture phase on the row before per-cell handlers fire.
  let contextMenuColumn: Column<ITEM> | undefined = $state(undefined);

  function dragStartHandler(e: DragEvent) {
    let element = e.target as HTMLElement;
    let rowId = element.getAttribute("data-row-id");

    if (!rowId) return;

    // Defense-in-depth: even though `draggable` is bound to isRowDraggable
    // and should already be false on ghost rows, suppress the drag start
    // here too so a stale browser attribute can't initiate a drag.
    const row = rows.find((r) => r.id === rowId);
    if (row?.isGhost) {
      e.preventDefault();
      return;
    }

    if (e.dataTransfer !== null) {
      e.dataTransfer.dropEffect = "move";
      draggedRowId = rowId;
    }
  }

  function dragEndHandler(e: DragEvent) {
    draggedRowId = null;
  }

  function dragOverHandler(e: DragEvent) {
    let rowId = findRowId(e.target as HTMLElement);

    if (rowId && draggedRowId && rowId !== draggedRowId) {
      currentDropRowId = rowId;
      e.preventDefault();
    }
  }

  function dragEnterHandler(e: DragEvent) {
    let rowId = findRowId(e.target as HTMLElement);

    if (rowId && draggedRowId && rowId !== draggedRowId) {
      currentDropRowId = rowId;
      e.preventDefault();
    }
  }

  function dragLeaveHandler(e: DragEvent) {
    let rowId = findRowId(e.target as HTMLElement);
    if (rowId === currentDropRowId) currentDropRowId = null;
    e.preventDefault();
  }

  function findRowId(e: HTMLElement): string | null {
    let id = e.getAttribute("data-row-id");
    if (id) return id;

    let parent = e.parentElement;
    if (parent) return findRowId(parent);

    return null;
  }

  function dropHandler(e: DragEvent) {
    let rowId = findRowId(e.target as HTMLElement);
    if (rowId) {
      props.onRowDragDrop?.(draggedRowId as string, rowId);

      dragLeaveHandler(e);
      e.preventDefault();
    }
  }

  function getRowClass(row: MRow<T>) {
    if (row.id === draggedRowId) return "outline-1 bg-primary-500";
    if (row.id === currentDropRowId) return "outline-2 bg-secondary-500";
    // Ghost rows are reflections of a primary occurrence elsewhere in the
    // tree. Render them muted (italic + lower-contrast text) and suppress
    // the hover background so they feel less interactive than real rows.
    if (row.isGhost) return "italic text-surface-500-500";
    if (row.isModified) return "bg-secondary-300-700";
    if (row.modifiedDescendent) return "bg-secondary-50-950";
    return "hover:bg-surface-100-900";
  }

  // Handles a left-click on a row. For ghost rows it routes the click to the
  // primary occurrence (scrolls it into view). For non-ghost rows it is a
  // no-op — TreeTable has no row-selection concept of its own. Clicks that
  // originated on an interactive child (button, link) are ignored so the
  // expander/link handlers stay authoritative.
  function handleRowClick(e: MouseEvent, row: MRow<T>) {
    if (!row.isGhost) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("button, a")) return;

    // Ghost rows are always work-item rows (groups are never ghosts —
    // see recipe_builder.rs). Look up the primary occurrence by the
    // underlying work-item id rather than the render-position key,
    // since two rows with the same WorkItemId now have distinct ids.
    if (row.data.type !== "workItem") return;
    const primaryId = findPrimaryRow(rows, row.data.workItemId)?.id;
    if (!primaryId) {
      console.debug(
        `[TreeTable] ghost click on ${row.id}: no primary occurrence in view`,
      );
      return;
    }

    const element = document.querySelector<HTMLElement>(
      `[data-row-id="${CSS.escape(primaryId)}"]`,
    );
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "center" });
    } else {
      console.debug(
        `[TreeTable] ghost click on ${row.id}: primary ${primaryId} not in DOM`,
      );
    }
  }

  let columnResize:
    | { startWidth: number; startX: number; index: number }
    | undefined = undefined;

  function handleColumnResizeOnPointerDown(event: PointerEvent) {
    const element = event.target as HTMLElement;
    const columnIndex = Number.parseInt(
      element.getAttribute("data-column-index")!
    );

    const columnElement = document.getElementById(
      `column-index-${columnIndex}`
    )!;

    columnResize = {
      startWidth: columnElement.getBoundingClientRect().width,
      startX: event.x,
      index: columnIndex,
    };

    element.setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function handleColumnResizeOnPointerUp(event: PointerEvent) {
    const element = event.target as HTMLElement;
    if (!element.hasPointerCapture(event.pointerId)) return;
    element.releasePointerCapture(event.pointerId);
    if (columnResize) {
      recordTelemetry({ event: "column_resize", column: columns[columnResize.index]?.name });
    }
    columnResize = undefined;
  }

  function handleColumnResizeOnPointerMove(event: PointerEvent) {
    const element = event.target as HTMLElement;
    if (!element.hasPointerCapture(event.pointerId)) return;

    if (!columnResize) return;

    let deltaX = event.x - columnResize.startX;

    let newSize = Math.max(columnResize.startWidth + deltaX, 20);

    columns[columnResize.index].width = `${newSize}px`;
    event.preventDefault();
  }

  function handleColumnResizeDoubleClick(event: MouseEvent) {
    const element = event.target as HTMLElement;
    const columnIndex = Number.parseInt(
      element.getAttribute("data-column-index")!
    );

    columns[columnIndex].width = "max-content";
  }

  let rowVisibilityObserver: IntersectionObserver | undefined = $state();

  const rowVisbilityObserverAttachment: Attachment = () => {
    rowVisibilityObserver = new IntersectionObserver(
      (intersections) => {
        intersections.forEach((intersection) => {
          const rowId = intersection.target.getAttribute("data-row-id");
          if (rowId) {
            if (intersection.isIntersecting) visibleRows.add(rowId);
            else visibleRows.delete(rowId);
          }
        });
      },
      { rootMargin: "500px" }
    );
  };

  const rowVisibilityChecker: Attachment = (element) => {
    rowVisibilityObserver?.observe(element);
    return () => {
      rowVisibilityObserver?.unobserve(element);
    };
  };

  type DivDragEvent = DragEvent & {
    currentTarget: EventTarget & HTMLDivElement;
  };

  let draggedColumnIndex = $state(-1);
  let droppedColumnIndex = $state(-1);

  function handleColumnHeaderDragStart(event: DivDragEvent) {
    let columnIndex = event.currentTarget.getAttribute("data-column-index");
    if (columnIndex === null) {
      console.log("WARNING: column drag started - but there's no index!");
      draggedColumnIndex = -1;
      return;
    }
    draggedColumnIndex = Number.parseInt(columnIndex);
    if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "move";
  }

  function handleColumnHeaderDrop(event: DivDragEvent) {
    if (draggedColumnIndex === -1) return;
    if (droppedColumnIndex === -1) return;

    let column = columns[draggedColumnIndex];
    columns.splice(draggedColumnIndex, 1);
    columns.splice(droppedColumnIndex, 0, column);

    recordTelemetry({ event: "column_reorder", column: column.name });

    droppedColumnIndex = -1;
    event.preventDefault();
  }

  function handleColumnHeaderDragOver(event: DivDragEvent) {
    if (draggedColumnIndex === -1) return;

    let columnIndex = event.currentTarget.getAttribute("data-column-index");
    if (columnIndex !== null) droppedColumnIndex = Number.parseInt(columnIndex);
    if (droppedColumnIndex === 0 || droppedColumnIndex === draggedColumnIndex) {
      droppedColumnIndex = -1;
      return;
    }

    event.preventDefault();
  }

  function handleColumnHeaderDragLeave(event: DivDragEvent) {
    droppedColumnIndex = -1;
  }
</script>

<!-- Component Container -->
<div
  class="overflow-x-auto overflow-y-scroll flex-1 min-h-0"
  {@attach rowVisbilityObserverAttachment}
>
  <div>
    {#if props.matchesFilter}
      <FindDialog bind:text={filterText} />
    {/if}

    <!-- Table container -->
    <div
      class="grid w-full"
      style={`grid-template-columns: ${gridTemplateColumns};`}
    >
      <!-- Header Row -->
      <div
        class="sticky top-0 grid grid-cols-subgrid h-fit"
        style={`grid-column: ${gridColumn};`}
      >
        {#each columns as column, index}
          <div
            id="column-index-{index}"
            class="text-lg font-bold bg-primary-100-900 text-primary-contrast-100-900 pl-1 flex justify-between"
            class:bg-secondary-500={index === droppedColumnIndex}
          >
            {@render columnHeader(column, index)}
            <div
              class="overflow-visible z-10 my-1 border-r border-r-surface-800-200"
            >
              <div
                data-column-index={index}
                class="w-[10px] left-[5px] h-full relative cursor-col-resize"
                onpointerdown={handleColumnResizeOnPointerDown}
                onpointerup={handleColumnResizeOnPointerUp}
                onpointermove={handleColumnResizeOnPointerMove}
                ondblclick={handleColumnResizeDoubleClick}
                ondragenter={handleColumnHeaderDragOver}
                ondragleave={handleColumnHeaderDragLeave}
                ondragover={handleColumnHeaderDragOver}
                ondrop={handleColumnHeaderDrop}
                role="separator"
                aria-orientation="vertical"
              ></div>
            </div>
            {#if index === columns.length - 1 && hiddenColumns.length > 0}
              <div class="left-[100%] absolute flex items-center h-full">
                <AddColumnButton
                  columns={hiddenColumns}
                  onColumnSelected={(column) => {
                    columns.push(column);
                    hiddenColumns = hiddenColumns.filter(
                      (c) => c.name !== column.name
                    );
                  }}
                />
              </div>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Rows -->
      {#each rows as row (row.id)}
        {@const modified = row.isModified}
        {@const modifiedDescendent = !modified && row.modifiedDescendent}
        {@const onRowFirstVisible = props.onRowFirstVisible}
        <TreeTableContextMenu
          getItems={() => props.getContextMenuItems(row, contextMenuColumn)}
        >
          {#snippet trigger({ props }: { props: any })}
            {@const menuOpen = props["data-state"] === "open"}
            <div
              {...props}
              class={[
                "grid-cols-subgrid grid overflow-hidden whitespace-nowrap border border-surface-200-800 cursor-default",
                menuOpen ? "outline-2 bg-primary-500" : getRowClass(row),
              ]}
              style={`padding-left: ${1 * row.level}rem; grid-column: ${gridColumn};`}
              draggable={isRowDraggable(row)}
              data-row-id={row.id}
              onclick={(e) => handleRowClick(e, row)}
              ondragstart={dragStartHandler}
              ondragend={dragEndHandler}
              ondragenter={dragEnterHandler}
              ondragleave={dragLeaveHandler}
              ondragover={dragOverHandler}
              ondrop={dropHandler}
              oncontextmenucapture={() => (contextMenuColumn = undefined)}
              {@attach onFirstVisible(row, onRowFirstVisible)}
              {@attach rowVisibilityChecker}
            >
              {#if row.isGroup}
                {@render groupRow(row)}
              {:else if visibleRows.has(row.id)}
                {@render itemRow(row)}
              {:else}
                &nbsp;
              {/if}
            </div>
          {/snippet}
        </TreeTableContextMenu>
      {/each}
    </div>
  </div>
</div>

{#snippet columnHeader(column: Column<ITEM>, index: number)}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="w-full flex justify-between overflow-hidden"
    class:bg-secondary-500={index === droppedColumnIndex}
    data-column-index={index}
    draggable={index > 0}
    ondragstart={handleColumnHeaderDragStart}
    ondragenter={handleColumnHeaderDragOver}
    ondragleave={handleColumnHeaderDragLeave}
    ondragover={handleColumnHeaderDragOver}
    ondrop={handleColumnHeaderDrop}
  >
    <div class="overflow-hidden text-ellipsis">
      {column.name.charAt(0).toUpperCase() + column.name.slice(1)}
    </div>
    {#if !column.disableMenu}
      <TableColumnMenu
        {column}
        onHideColumn={(column: Column<ITEM>) => {
          columns = columns.filter((c) => c.name != column.name);
          hiddenColumns.push(column);
        }}
      />
    {/if}
  </div>
{/snippet}

{#snippet groupRow(row: Row<T>)}
  <div class="py-2 font-bold">
    {@render expander(row)}
    {@render props.renderGroup(props.getGroup(row))}
  </div>
{/snippet}

{#snippet itemRow(row: Row<T>)}
  {#each columns as column, index}
    {@const item = props.getItem(row)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={[
        "overflow-hidden border-r border-surface-200-800 py-0.5",
        index === 0 && "pr-1 flex gap-1 flex-nowrap",
        index !== 0 && "px-1 overflow-ellipsis ",
      ]}
      oncontextmenucapture={() => (contextMenuColumn = column)}
    >
      {#if index === 0}
        {@render expander(row)}
      {/if}
      {#if item}
        {@render column.render(item)}
      {:else}
        &nbsp;
      {/if}
    </div>
  {/each}
{/snippet}

{#snippet expander(row: Row<T>)}
  {#if row.hasChildren}
    <button
      class="shrink-0"
      oncontextmenucapture={() => (contextMenuColumn = undefined)}
      onclick={() => {
        if (expanded.includes(row.id)) {
          expanded = expanded.filter((i) => i !== row.id);
          recordTelemetry({ event: "expand_collapse", action: "collapse" });
        } else {
          expanded.push(row.id);
          recordTelemetry({ event: "expand_collapse", action: "expand" });
        }
      }}
    >
      {#if expanded.includes(row.id)}
        <ChevronDown size="1em" class="hover:bg-primary-500" />
      {:else}
        <ChevronRight size="1em" class="hover:bg-primary-500" />
      {/if}
    </button>
  {:else}
    <div class="shrink-0 inline-block size-[1em]">&nbsp;</div>
  {/if}
{/snippet}

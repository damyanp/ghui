<script lang="ts" module>
  export type Column<ITEM> = {
    name: string;
    width: string;
    render: Snippet<[ITEM]>;
    disableMenu?: boolean;
    getMenuIconSVG?: (column: Column<ITEM>) => string | undefined;
    renderMenuContent?: Snippet<[Column<ITEM>]>;
  };

  type Row<T> = {
    level: number;
    id: string;
    hasChildren: boolean;
    isModified: boolean;
    isGroup: boolean;
    data: T;
  };

  type Props<T, GROUP, ITEM> = {
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
    getContextMenuItems: (row: Row<T>) => MenuItem[];
    onRowDragDrop?: (draggedRowId: string, droppedOntoRowId: string) => void;
    onRowFirstVisible?: (row: Row<T>) => void;
    matchesFilter?: (row: Row<T>, filter: string) => boolean;
  };
</script>

<script lang="ts" generics="T, GROUP, ITEM">
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

  let {
    columns = $bindable(),
    hiddenColumns = $bindable(),
    expanded = $bindable([]),
    visibleRows = $bindable(new SvelteSet<string>()),
    ...props
  }: Props<T, GROUP, ITEM> = $props();

  type MRow<T> = Row<T> & { modifiedDescendent: boolean };

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

  function dragStartHandler(e: DragEvent) {
    let element = e.target as HTMLElement;
    let rowId = element.getAttribute("data-row-id");

    if (rowId && e.dataTransfer !== null) {
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
    if (row.isModified) return "bg-secondary-300-700";
    if (row.modifiedDescendent) return "bg-secondary-50-950";
    return "hover:bg-surface-100-900";
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
  class="px-5 my-5 overflow-x-auto overflow-y-scroll"
  {@attach rowVisbilityObserverAttachment}
>
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
          class="text-lg font-bold bg-surface-300-700 text-surface-contrast-300-700 pl-1 flex justify-between"
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
      <TreeTableContextMenu getItems={() => props.getContextMenuItems(row)}>
        {#snippet trigger({ props }: { props: any })}
          {@const menuOpen = props["data-state"] === "open"}
          <div
            {...props}
            class={[
              "grid-cols-subgrid grid overflow-hidden whitespace-nowrap border border-surface-200-800 cursor-default",
              menuOpen ? "outline-2 bg-primary-500" : getRowClass(row),
            ]}
            style={`padding-left: ${1 * row.level}rem; grid-column: ${gridColumn};`}
            draggable={!row.isGroup}
            data-row-id={row.id}
            ondragstart={dragStartHandler}
            ondragend={dragEndHandler}
            ondragenter={dragEnterHandler}
            ondragleave={dragLeaveHandler}
            ondragover={dragOverHandler}
            ondrop={dropHandler}
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
    <div
      class={[
        "overflow-hidden border-r border-surface-200-800 py-0.5",
        index === 0 && "pr-1 flex gap-1 flex-nowrap",
        index !== 0 && "px-1 overflow-ellipsis ",
      ]}
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
      onclick={() => {
        if (expanded.includes(row.id)) {
          expanded = expanded.filter((i) => i !== row.id);
        } else {
          expanded.push(row.id);
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

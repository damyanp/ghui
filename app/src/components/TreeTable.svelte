<script lang="ts" generics="T, GROUP, ITEM">
  import { ChevronDown, ChevronRight } from "@lucide/svelte";
  import { ContextMenu } from "bits-ui";
  import { tick, type Snippet } from "svelte";
  import TreeTableContextMenu, {
    type MenuItem,
  } from "./TreeTableContextMenu.svelte";
  import { onFirstVisible } from "$lib/OnVirstVisible";
  import TableColumnHeader from "./TableSingleSelectColumnHeader.svelte";

  type Row<T> = {
    level: number;
    id: string;
    hasChildren: boolean;
    isModified: boolean;
    isGroup: boolean;
    data: T;
  };

  type Column = {
    name: string;
    width: string;
    render: Snippet<[ITEM]>;
    renderHeader?: Snippet<[string]>;
  };

  type Props = {
    rows: Row<T>[];
    columns: Column[];
    expanded?: string[];
    getGroup: (row: Row<T>) => GROUP;
    getItem: (row: Row<T>) => ITEM | undefined;
    renderGroup: Snippet<[GROUP]>;
    getContextMenuItems: (row: Row<T>) => MenuItem[];
    onRowDragDrop?: (draggedRowId: string, droppedOntoRowId: string) => void;
    onRowFirstVisible?: (row: Row<T>) => void;
  };

  let {
    columns = $bindable(),
    expanded = $bindable([]),
    ...props
  }: Props = $props();

  type MRow<T> = Row<T> & { modifiedDescendent: boolean };

  const rows: MRow<T>[] = $derived.by(() => {
    let rows: MRow<T>[] = [];

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
    if (rowId && rowId !== draggedRowId) {
      currentDropRowId = rowId;
      e.preventDefault();
    }
  }

  function dragEnterHandler(e: DragEvent) {
    let rowId = findRowId(e.target as HTMLElement);
    currentDropRowId = rowId;
    e.preventDefault();
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
</script>

<!-- Component Container -->
<div class="px-5 my-5 overflow-x-auto overflow-y-scroll">
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
        >
          {#if column.renderHeader}
            {@render column.renderHeader(column.name)}
          {:else}
            {column.name}
          {/if}
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
              role="separator"
              aria-orientation="vertical"
            ></div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Rows -->
    {#each rows as row (row.id)}
      {@const modified = row.isModified}
      {@const modifiedDescendent = !modified && row.modifiedDescendent}
      {@const unmodified = !(modified || modifiedDescendent)}
      {@const onRowFirstVisible = props.onRowFirstVisible}
      <TreeTableContextMenu items={props.getContextMenuItems(row)}>
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
          >
            {#if row.isGroup}
              {@render groupRow(row)}
            {:else}
              {@render itemRow(row)}
            {/if}
          </div>
        {/snippet}
      </TreeTableContextMenu>
    {/each}
  </div>
</div>

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

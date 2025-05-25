<script lang="ts" generics="T, GROUP, ITEM">
  import { ChevronDown, ChevronRight } from "@lucide/svelte";
  import { ContextMenu } from "bits-ui";
  import type { Snippet } from "svelte";
  import TreeTableContextMenu, {
    type MenuItem,
  } from "./TreeTableContextMenu.svelte";

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
  };

  type Props = {
    rows: Row<T>[];
    columns: Column[];
    getGroup: (row: Row<T>) => GROUP;
    getItem: (row: Row<T>) => ITEM;
    renderGroup: Snippet<[GROUP]>;
    getContextMenuItems: (row: Row<T>) => MenuItem[];
  };
  let props: Props = $props();

  type MRow<T> = Row<T> & { modifiedDescendent: boolean };

  let expanded = $state<string[]>([]);

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

  const gridTemplateColumns = $derived.by(() => {
    return props.columns.map((c) => c.width).join(" ");
  });

  const gridColumn = $derived(
    `span ${props.columns.length} / span ${props.columns.length};`
  );
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
      {#each props.columns as column}
        <div
          class="text-lg font-bold bg-surface-300-700 text-surface-contrast-300-700"
        >
          {column.name}
        </div>
      {/each}
    </div>

    <!-- Rows -->
    {#each rows as row (row.id)}
      {@const modified = row.isModified}
      {@const modifiedDescendent = !modified && row.modifiedDescendent}
      {@const unmodified = !(modified || modifiedDescendent)}
      <TreeTableContextMenu items={props.getContextMenuItems(row)}>
        {#snippet trigger({ props }: { props: any })}
          {@const menuOpen = props["data-state"] === "open"}
          <div
            {...props}
            class={[
              "grid-cols-subgrid grid col-span-9 overflow-hidden whitespace-nowrap border border-surface-200-800",
              modified && "bg-secondary-300-700",
              modifiedDescendent && "bg-secondary-50-950",
              unmodified && "hover:bg-surface-100-900",
              menuOpen && "outline-2 bg-primary-500",
            ]}
            style={`padding-left: ${1 * row.level}rem;`}
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
  {#each props.columns as column, index}
    {@const item = props.getItem(row)}
    <div
      class={[
        "overflow-hidden border-r border-surface-200-800 py-0.5",
        index === 0 && "flex gap-1 flex-nowrap",
        index !== 0 && "px-1 overflow-ellipsis ",
      ]}
    >
      {#if index === 0}
        {@render expander(row)}
      {/if}
      {@render column.render(item)}
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

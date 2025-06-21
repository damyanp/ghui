<script lang="ts" generics="ITEM">
  import * as octicons from "@primer/octicons";
  import { Popover } from "@skeletonlabs/skeleton-svelte";
  import type { Column } from "./TreeTable.svelte";

  type Props = {
    column: Column<ITEM>;
    onHideColumn?: (column: Column<ITEM>) => void;
  };

  const props: Props = $props();

  function getMenuIconSVG<ITEM>(column: Column<ITEM>): string {
    const svg = column.getMenuIconSVG?.(column);
    if (svg) return svg;

    return octicons["kebab-horizontal"].toSVG();
  }

  let open = $state(false);
  $effect(() => {});
</script>

<Popover
  {open}
  onOpenChange={(d) => (open = d.open)}
  arrow
  arrowBackground="!bg-surface-100 dark:!bg-surface-900"
  contentBase="card bg-surface-100-900 p-4 space-y-4 "
>
  {#snippet trigger()}
    <div class="hover:bg-surface-200-800 m-0.5 px-1 rounded-full flex-none">
      {@html getMenuIconSVG(props.column)}
    </div>
  {/snippet}

  {#snippet content()}
    <div class="flex flex-col gap-2">
      {#if props.onHideColumn}
        <div class="w-full rounded bg-surface-200-800 p-2 flex justify-center">
          <button
            class="btn btn-sm preset-tonal"
            onclick={() => {
              open = false;
              props.onHideColumn?.(props.column);
            }}
          >
            Hide Column
          </button>
        </div>
      {/if}
      {#if props.column.renderMenuContent}
        <div>
          {@render props.column.renderMenuContent(props.column)}
        </div>
      {/if}
    </div>
  {/snippet}
</Popover>

<script lang="ts" generics="ITEM">
  import * as octicons from "@primer/octicons";
  import { Popover } from "@skeletonlabs/skeleton-svelte";
  import type { Column } from "./TreeTable.svelte";

  type Props = {
    column: Column<ITEM>;
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
  contentBase="card bg-surface-100-900 p-4 space-y-4 max-w-[320px] "
>
  {#snippet trigger()}
    <div class="hover:bg-surface-200-800 m-0.5 px-1 rounded-full flex-none">
      {@html getMenuIconSVG(props.column)}
    </div>
  {/snippet}

  {#snippet content()}
    <div>
      {#if props.column.renderMenuContent}
        {@render props.column.renderMenuContent(props.column)}
      {/if}
    </div>
  {/snippet}
</Popover>

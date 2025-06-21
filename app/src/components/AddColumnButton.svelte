<script lang="ts" generics="ITEM">
  import { CirclePlus } from "@lucide/svelte";
  import { Popover } from "@skeletonlabs/skeleton-svelte";
  import type { Column } from "./TreeTable.svelte";

  type Props = {
    columns: Column<ITEM>[];
    onColumnSelected: (column: Column<ITEM>) => void;
  };

  let { columns, onColumnSelected }: Props = $props();
  let open = $state(false);
</script>

<Popover
  {open}
  onOpenChange={(o) => (open = o.open)}
  classes="flex items-center"
  triggerClasses="btn btn-icon btn-icon-s m-0 p-1"
  arrow
  arrowBackground="!bg-surface-100 dark:!bg-surface-900"
  contentBase="card bg-surface-100-900 p-4 space-y-4 max-w-[320px] "
>
  {#snippet trigger()}
    <CirclePlus />
  {/snippet}

  {#snippet content()}
    <div class="grid grid-cols-1 gap-2 max-h-[50vh] overflow-y-auto">
      {#each columns as column}
        <button
          class="btn preset-tonal justify-start"
          onclick={() => {
            open = false;
            onColumnSelected(column);
          }}
        >
          {column.name}
        </button>
      {/each}
    </div>
  {/snippet}
</Popover>

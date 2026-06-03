<script lang="ts" generics="ITEM">
  import { CirclePlus } from "@lucide/svelte";
  import { Popover, Portal } from "@skeletonlabs/skeleton-svelte";
  import type { Column } from "./TreeTable.svelte";

  type Props = {
    columns: Column<ITEM>[];
    onColumnSelected: (column: Column<ITEM>) => void;
  };

  let { columns, onColumnSelected }: Props = $props();
  let open = $state(false);
</script>

<Popover {open} onOpenChange={(o) => (open = o.open)}>
  <Popover.Trigger class="btn btn-icon btn-icon-s m-0 p-1 flex items-center">
    <CirclePlus />
  </Popover.Trigger>

  <Portal>
    <Popover.Positioner>
      <Popover.Content
        class="card bg-surface-100-900 p-4 space-y-4 max-w-[320px]"
      >
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
      </Popover.Content>
    </Popover.Positioner>
  </Portal>
</Popover>

<script lang="ts" generics="ITEM">
  import * as octicons from "@primer/octicons";
  import { Popover, Portal } from "@skeletonlabs/skeleton-svelte";
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

<Popover {open} onOpenChange={(d) => (open = d.open)}>
  <Popover.Trigger
    class="hover:bg-surface-200-800 m-0.5 px-1 rounded-full flex-none"
  >
    {@html getMenuIconSVG(props.column)}
  </Popover.Trigger>

  <Portal>
    <Popover.Positioner>
      <Popover.Content class="card bg-surface-100-900 p-4 space-y-4">
        <Popover.Arrow
          class="[--arrow-size:8px] [--arrow-background:var(--color-surface-100)] dark:[--arrow-background:var(--color-surface-900)]"
        >
          <Popover.ArrowTip />
        </Popover.Arrow>
        <div class="flex flex-col gap-2">
          {#if props.onHideColumn}
            <div
              class="w-full rounded bg-surface-200-800 p-2 flex justify-center"
            >
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
      </Popover.Content>
    </Popover.Positioner>
  </Portal>
</Popover>

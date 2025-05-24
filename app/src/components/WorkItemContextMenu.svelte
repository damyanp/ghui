<script lang="ts">
  import { ContextMenu, type ContextMenuRootProps } from "bits-ui";
  import type { Snippet } from "svelte";

  export type MenuOption =
    | { type: "text"; title: string }
    | { type: "action"; title: string; action: () => void };

  type Props = ContextMenuRootProps & { trigger: Snippet; items: MenuOption[] };

  let { trigger, items, ...rootProps }: Props = $props();
</script>

<ContextMenu.Root {...rootProps}>
  <ContextMenu.Trigger>
    {@render trigger()}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="p-3 border rounded bg-surface-900">
      {#each items as item}
        {#if item.type === "text"}
          <ContextMenu.Item class="cursor-default">{item.title}</ContextMenu.Item>
        {:else}
          <ContextMenu.Item>
            <button
              class="p-2 btn preset-tonal-surface"
              onclick={() => item.action()}
            >
              {item.title}
            </button>
          </ContextMenu.Item>
        {/if}
      {/each}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>

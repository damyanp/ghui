<script lang="ts">
  import { ContextMenu, type ContextMenuRootProps } from "bits-ui";
  import type { Snippet } from "svelte";

  export type MenuItem =
    | { type: "text"; title: string }
    | { type: "action"; title: string; action: () => void };

  type Props = ContextMenuRootProps & {
    trigger: Snippet<[{ props: any }]>;
    getItems?: () => MenuItem[];
  };

  let { trigger, getItems, ...rootProps }: Props = $props();
</script>

{#if getItems}
  <ContextMenu.Root {...rootProps}>
    <ContextMenu.Trigger child={trigger} />
    <ContextMenu.Portal>
      <ContextMenu.Content class="p-3 border rounded bg-surface-900">
        {#each getItems() as item}
          {#if item.type === "text"}
            <ContextMenu.Item class="cursor-default"
              >{item.title}</ContextMenu.Item
            >
          {:else}
            <button
              class="p-2 btn preset-tonal surface"
              onclick={() => item.action()}>{item.title}</button
            >
          {/if}
        {/each}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>
{:else}
  {@render trigger({ props: {} })}
{/if}

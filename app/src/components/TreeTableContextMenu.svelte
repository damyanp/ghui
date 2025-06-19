<script lang="ts">
  import { ContextMenu, type ContextMenuRootProps } from "bits-ui";
  import type { Snippet } from "svelte";

  export type MenuItem =
    | { type: "text"; title: string }
    | { type: "action"; title: string; action: () => void }
    | { type: "link"; title: string; href: string };

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
            <ContextMenu.Item class="cursor-default text-lg">
              {item.title}
            </ContextMenu.Item>
          {:else if item.type === "action"}
            <ContextMenu.Item
              class="cursor-default hover:bg-primary-50-950"
              onSelect={() => item.action()}
            >
              {item.title}
            </ContextMenu.Item>
          {:else if item.type === "link"}
            <ContextMenu.Item>
              <a
                class="text-blue-400 underline whitespace-nowrap shrink-0 block"
                target="_blank"
                href={item.href}
              >
                {item.title}
              </a>
            </ContextMenu.Item>
          {/if}
        {/each}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>
{:else}
  {@render trigger({ props: {} })}
{/if}

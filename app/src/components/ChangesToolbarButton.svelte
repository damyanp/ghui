<script lang="ts">
  import type { Changes } from "$lib/bindings/Changes";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { Eye, EyeOff, Save, Trash2 } from "@lucide/svelte";
  import { scale } from "svelte/transition";

  const context = getWorkItemContext();

  const numChanges = $derived(Object.keys(context.data.changes.data).length);
</script>

{#if numChanges}
  <div
    class="cursor-default rounded-2xl w-fit bg-primary-50-950 text-xs h-full flex flex-row p-1 items-center"
    transition:scale
  >
    <button
      class="btn p-1"
      title="Delete Changes"
      onclick={async () => await context.deleteChanges()}
    >
      <Trash2 />
    </button>
    <button class="btn p-1" title="Save">
      <Save />
    </button>

    <div class="border-surface-100-900 border-r p-1 align-middle">
      {numChanges} changes
    </div>

    <button
      title="Preview"
      aria-pressed={context.previewChanges}
      onclick={() => {
        context.setPreviewChanges(!context.previewChanges);
      }}
      class="btn p-1"
    >
      {#if context.previewChanges}
        <Eye />
      {:else}
        <EyeOff />
      {/if}
    </button>
  </div>
{/if}

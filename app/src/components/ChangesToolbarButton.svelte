<script lang="ts">
  import type { Changes } from "$lib/bindings/Changes";
  import { Eye, EyeOff, Save, Trash2 } from "@lucide/svelte";

  let { changes }: { changes: Changes } = $props();

  const numChanges = $derived(Object.keys(changes.data).length);

  let previewChanges = $state(true);
</script>

{#if numChanges}
  <div
    class="cursor-default rounded-2xl w-fit bg-primary-50-950 text-xs h-full flex flex-row p-1 items-center"
  >
    <button class="btn p-1" title="Delete Changes">
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
      aria-pressed={previewChanges}
      onclick={() => (previewChanges = !previewChanges)}
      class="btn p-1"
    >
      {#if previewChanges}
        <Eye />
      {:else}
        <EyeOff />
      {/if}
    </button>
  </div>
{/if}

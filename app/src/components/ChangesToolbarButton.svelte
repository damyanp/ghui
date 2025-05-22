<script lang="ts">
  import { getWorkItemContext, makeProgressChannel } from "$lib/WorkItemContext.svelte";
  import { Eye, EyeOff, Save, Trash2 } from "@lucide/svelte";
  import { scale } from "svelte/transition";

  const context = getWorkItemContext();

  const numChanges = $derived(Object.keys(context.data.changes.data).length);

  let saveProgress = $state(0);

  async function saveChanges() {
    if (saveProgress !== 0) return;

    const progress = makeProgressChannel((value) => (saveProgress = value));
    await context.saveChanges(progress);
    saveProgress = 0;
  }

  let buttonClasses = $derived.by(() => {
    let classes = ["btn", "p-1"];
    if (saveProgress > 0) {
      classes.push("text-primary-100-900");
    }
    return classes;
  });

  let progressStyle = $derived.by(() => {
    let percent = `${(1 - saveProgress) * 100}%`;
    let bg = "transparent";
    let fg = "blue";

    return `background-image: linear-gradient(90deg,${bg},${percent},${bg},${percent},${fg})`;
  });
</script>

{#if numChanges}
  <div
    class="cursor-default rounded-2xl w-fit bg-primary-50-950 text-xs h-full flex flex-row p-1 items-center bg-linear-[90deg,transparent,20%,transparent,20%,blue"
    style={progressStyle}
    transition:scale
  >
    <button
      class={buttonClasses}
      title="Delete Changes"
      onclick={async () => await context.deleteChanges()}
    >
      <Trash2 />
    </button>
    <button class={buttonClasses} title="Save" onclick={saveChanges}>
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
      class={buttonClasses}
    >
      {#if context.previewChanges}
        <Eye />
      {:else}
        <EyeOff />
      {/if}
    </button>
  </div>
{/if}

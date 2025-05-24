<script lang="ts">
  import {
    getWorkItemContext,
    makeProgressChannel,
  } from "$lib/WorkItemContext.svelte";
  import { Eye, EyeOff, Save, Trash2 } from "@lucide/svelte";
  import { scale } from "svelte/transition";
  import AppBarButton from "./AppBarButton.svelte";

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
    class="cursor-default rounded-2xl w-fit text-xs h-full flex flex-row p-1 px-2 mx-2 border items-center bg-linear-[90deg,transparent,20%,transparent,20%,blue"
    style={progressStyle}
    transition:scale
  >
    <span class="self-start h-full text-xs border-r pe-1"
      >{numChanges} change{numChanges > 1 ? "s" : ""}</span
    >
    <AppBarButton
      icon={Trash2}
      text="Discard"
      onclick={async () => await context.deleteChanges()}
    />

    <AppBarButton icon={Save} text="Save" onclick={saveChanges} />

    <AppBarButton
      text="Preview"
      icon={context.previewChanges ? Eye : EyeOff}
      onclick={() => {
        context.setPreviewChanges(!context.previewChanges);
      }}
    />
  </div>
{/if}

<script lang="ts">
  import {
    getWorkItemContext,
    makeProgressChannel,
  } from "$lib/WorkItemContext.svelte";
  import { Eye, EyeOff, ReceiptText, Save, Trash2, Undo2, Redo2 } from "@lucide/svelte";
  import AppBarButton from "./AppBarButton.svelte";
  import PendingChangesDialog from "./PendingChangesDialog.svelte";

  const context = getWorkItemContext();

  const numChanges = $derived(Object.keys(context.data.changes.data).length);
  const canUndo = $derived(context.data.canUndo);
  const canRedo = $derived(context.data.canRedo);

  let saveProgress = $state(0);

  let pendingChangesOpen = $state(false);

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

<div
  class="cursor-default rounded-2xl w-fit text-xs h-full flex flex-row p-1 px-2 mx-2 border items-center bg-linear-[90deg,transparent,20%,transparent,20%,blue"
  style={progressStyle}
>
  <span class="self-start h-full text-xs border-r pe-1"
    >{numChanges} change{numChanges !== 1 ? "s" : ""}</span
  >

  <AppBarButton
    icon={Undo2}
    text="Undo"
    disabled={!canUndo}
    onclick={async () => await context.undoChange()}
  />

  <AppBarButton
    icon={Redo2}
    text="Redo"
    disabled={!canRedo}
    onclick={async () => await context.redoChange()}
  />

  <AppBarButton
    icon={Trash2}
    text="Discard"
    disabled={!numChanges}
    onclick={async () => await context.deleteChanges()}
  />

  <AppBarButton
    icon={Save}
    text="Save"
    disabled={!numChanges}
    onclick={saveChanges}
  />

  <AppBarButton
    text="Preview"
    icon={context.previewChanges ? Eye : EyeOff}
    disabled={!numChanges}
    onclick={() => {
      context.setPreviewChanges(!context.previewChanges);
    }}
  />

  <AppBarButton
    text="Details"
    icon={ReceiptText}
    disabled={!numChanges}
    onclick={() => {
      pendingChangesOpen = true;
    }}
  />

  <PendingChangesDialog bind:open={pendingChangesOpen} />
</div>

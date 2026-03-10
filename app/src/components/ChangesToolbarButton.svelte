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

  let saveStyle = $derived.by(() => {
    if (saveProgress === 0) return "";
    let percent = `${(1 - saveProgress) * 100}%`;
    let bg = "transparent";
    let fg = "blue";

    return `background-image: linear-gradient(90deg,${bg},${percent},${bg},${percent},${fg})`;
  });
</script>

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
  style={saveStyle}
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
  badge={numChanges > 0 ? numChanges : undefined}
  onclick={() => {
    pendingChangesOpen = true;
  }}
/>

<PendingChangesDialog bind:open={pendingChangesOpen} />

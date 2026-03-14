<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import WorkItemTree from "../components/WorkItemTree.svelte";
  import RefreshButton from "../components/RefreshButton.svelte";
  import {
    setWorkItemContext,
    WorkItemContext,
    makeProgressChannel,
  } from "$lib/WorkItemContext.svelte";
  import SanitizeButton from "../components/SanitizeButton.svelte";
  import {
    ChartGantt,
    Eye,
    EyeOff,
    ListTree,
    Redo2,
    ReceiptText,
    Save,
    Trash2,
    Undo2,
  } from "@lucide/svelte";
  import AppBarButton from "../components/AppBarButton.svelte";
  import PendingChangesDialog from "../components/PendingChangesDialog.svelte";
  import WorkItemExecutionTracker, {
    setWorkItemExecutionTrackerContext,
    WorkItemExecutionTrackerContext,
  } from "../components/WorkItemExecutionTracker.svelte";

  const context = setWorkItemContext(new WorkItemContext());
  setWorkItemExecutionTrackerContext(new WorkItemExecutionTrackerContext());

  type Mode = "items" | "xtracker";
  let mode = $state<Mode>("items");

  const itemsIconClass = $derived(mode === "items" ? "bg-primary-500" : "");
  const xtrackerIconClass = $derived(
    mode === "xtracker" ? "bg-primary-500" : ""
  );

  // Changes toolbar state
  const numChanges = $derived(Object.keys(context.data.changes.data).length);
  const canUndo = $derived(context.data.canUndo);
  const canRedo = $derived(context.data.canRedo);

  let saveProgress = $state(0);
  let pendingChangesOpen = $state(false);
  let busy = $state(false);
  const disabled = $derived(busy || context.loadProgress > 0);

  async function runBusy(action: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await action();
    } finally {
      busy = false;
    }
  }

  async function onRefreshClicked(): Promise<void> {
    await runBusy(() => context.refresh());
  }

  async function saveChanges() {
    await runBusy(async () => {
      try {
        const progress = makeProgressChannel(
          (value) => (saveProgress = value)
        );
        await context.saveChanges(progress);
      } finally {
        saveProgress = 0;
      }
    });
  }

  let saveStyle = $derived.by(() => {
    if (saveProgress === 0) return "";
    let percent = `${(1 - saveProgress) * 100}%`;
    let bg = "transparent";
    let fg = "blue";
    return `background-image: linear-gradient(90deg,${bg},${percent},${bg},${percent},${fg})`;
  });
</script>

<div class="flex flex-col h-full w-full fixed">
  <AppBar padding="px-4 py-1">
    {#snippet lead()}
      <div
        class="content-center h-full text-lg font-black border-r rounded-2xl pe-1"
      >
        ghui
      </div>
      <RefreshButton
        progress={context.loadProgress}
        disabled={disabled}
        onclick={onRefreshClicked}
      />

      <div class="w-3"></div>

      <AppBarButton
        icon={Save}
        text="Save"
        disabled={!numChanges || disabled}
        style={saveStyle}
        onclick={saveChanges}
      />
      <AppBarButton
        icon={Trash2}
        text="Discard"
        disabled={!numChanges || disabled}
        onclick={() => runBusy(() => context.deleteChanges())}
      />

      <div class="w-3"></div>

      <AppBarButton
        text="Details"
        icon={ReceiptText}
        disabled={!numChanges || disabled}
        badge={numChanges > 0 ? numChanges : undefined}
        onclick={() => {
          pendingChangesOpen = true;
        }}
      />
      <AppBarButton
        text="Preview"
        icon={context.previewChanges ? Eye : EyeOff}
        disabled={!numChanges || disabled}
        onclick={() => {
          context.setPreviewChanges(!context.previewChanges);
        }}
      />

      <div class="w-3"></div>

      <SanitizeButton
        disabled={disabled}
        onclick={() => runBusy(() => context.sanitize())}
      />

      <div class="w-3"></div>

      <AppBarButton
        icon={Undo2}
        text="Undo"
        disabled={!canUndo || disabled}
        onclick={() => runBusy(() => context.undoChange())}
      />
      <AppBarButton
        icon={Redo2}
        text="Redo"
        disabled={!canRedo || disabled}
        onclick={() => runBusy(() => context.redoChange())}
      />

      <div class="w-3"></div>

      <AppBarButton
        text="Items"
        icon={ListTree}
        iconClass={itemsIconClass}
        disabled={disabled}
        onclick={() => {
          mode = "items";
        }}
      />
      <AppBarButton
        text="X-tracker"
        icon={ChartGantt}
        iconClass={xtrackerIconClass}
        disabled={disabled}
        onclick={() => {
          mode = "xtracker";
        }}
      />
    {/snippet}

    {#snippet trail()}
      <Pat />
    {/snippet}
  </AppBar>

  <PendingChangesDialog bind:open={pendingChangesOpen} />

  {#if mode === "items"}
    <WorkItemTree />
  {:else if mode === "xtracker"}
    <WorkItemExecutionTracker />
  {:else}
    <h1>Unknown mode {mode}</h1>
  {/if}
</div>

<script lang="ts">
  import { AppBar } from "@skeletonlabs/skeleton-svelte";
  import Pat from "../components/Pat.svelte";
  import WorkItemTree from "../components/WorkItemTree.svelte";
  import RefreshButton from "../components/RefreshButton.svelte";
  import {
    setWorkItemContext,
    WorkItemContext,
    makeProgressChannel,
    recordTelemetry,
  } from "$lib/WorkItemContext.svelte";
  import {
    Bubbles,
    ChartGantt,
    Eye,
    EyeOff,
    ListTree,
    LinkIcon,
    Redo2,
    ReceiptText,
    Save,
    ScrollText,
    Trash2,
    Undo2,
    ArrowDownToLine,
  } from "@lucide/svelte";
  import AppBarButton from "../components/AppBarButton.svelte";
  import LogPanel from "../components/LogPanel.svelte";
  import PendingChangesDialog from "../components/PendingChangesDialog.svelte";
  import AddItemDialog from "../components/AddItemDialog.svelte";
  import WorkItemExecutionTracker, {
    setWorkItemExecutionTrackerContext,
    WorkItemExecutionTrackerContext,
  } from "../components/WorkItemExecutionTracker.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { ReleaseInfo } from "$lib/bindings/ReleaseInfo";

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
  let addItemDialogOpen = $state(false);
  let logPanelOpen = $state(false);
  let busy = $state(false);
  const disabled = $derived(busy || context.loadProgress > 0);

  // Update check state
  let updateInfo = $state<ReleaseInfo | null>(null);
  let updateCheckState = $state<"idle" | "checking" | "downloading">("idle");

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

  async function checkForUpdate() {
    updateCheckState = "checking";
    updateInfo = null;
    try {
      updateInfo = await invoke<ReleaseInfo | null>("check_for_update");
    } catch {
      updateInfo = null;
    } finally {
      updateCheckState = "idle";
    }
  }

  async function installUpdate() {
    if (!updateInfo) return;
    updateInfo = null;
    updateCheckState = "downloading";
    try {
      await invoke("install_update");
      // App exits inside install_update after spawning the installer.
    } catch {
      // Intentionally empty: state is reset in finally.
    } finally {
      updateCheckState = "idle";
    }
  }

  const updateButtonText = $derived.by(() => {
    if (updateCheckState === "checking") return "Checking…";
    if (updateCheckState === "downloading") return "Downloading…";
    if (updateInfo) return `Install ${updateInfo.tagName}`;
    return "Updates";
  });

  const updateButtonDisabled = $derived(
    updateCheckState !== "idle" || disabled
  );

  const updateIconClass = $derived(updateInfo ? "bg-primary-500" : "");

  const onUpdateClicked = $derived(updateInfo ? installUpdate : checkForUpdate);
</script>

<div class="flex flex-col h-full w-full fixed">
  <AppBar padding="px-4 py-1">
    {#snippet lead()}
      <div
        class="content-center h-full pe-1"
      >
        <img src="/icon.svg" alt="ghui" class="h-12" />
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
          recordTelemetry({ event: "pending_changes_opened" });
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

      <AppBarButton
        icon={LinkIcon}
        text="Add"
        disabled={disabled}
        onclick={() => {
          addItemDialogOpen = true;
        }}
      />

      <AppBarButton
        icon={Bubbles}
        text="Sanitize"
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
          if (mode !== "items") {
            recordTelemetry({ event: "mode_switched", to: "items" });
          }
          mode = "items";
        }}
      />
      <AppBarButton
        text="X-tracker"
        icon={ChartGantt}
        iconClass={xtrackerIconClass}
        disabled={disabled}
        onclick={() => {
          if (mode !== "xtracker") {
            recordTelemetry({ event: "mode_switched", to: "xtracker" });
          }
          mode = "xtracker";
        }}
      />

      <div class="w-3"></div>

      <AppBarButton
        text="Output"
        icon={ScrollText}
        badge={context.unreadErrorCount > 0
          ? context.unreadErrorCount
          : undefined}
        onclick={() => {
          logPanelOpen = !logPanelOpen;
          recordTelemetry({ event: "log_panel_toggled", open: logPanelOpen });
          if (logPanelOpen) {
            context.markErrorsAsRead();
          }
        }}
      />
    {/snippet}

    {#snippet trail()}
      <AppBarButton
        text={updateButtonText}
        icon={ArrowDownToLine}
        iconClass={updateIconClass}
        disabled={updateButtonDisabled}
        onclick={onUpdateClicked}
      />
      <Pat />
    {/snippet}
  </AppBar>

  <PendingChangesDialog bind:open={pendingChangesOpen} />
  <AddItemDialog bind:open={addItemDialogOpen} />

  <div class="flex flex-col flex-1 min-h-0 overflow-hidden">
    {#if mode === "items"}
      <WorkItemTree />
    {:else if mode === "xtracker"}
      <WorkItemExecutionTracker />
    {:else}
      <h1>Unknown mode {mode}</h1>
    {/if}
  </div>

  {#if logPanelOpen}
    <LogPanel
      onclose={() => {
        logPanelOpen = false;
      }}
    />
  {/if}
</div>

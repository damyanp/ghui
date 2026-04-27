<script lang="ts">
  import { onDestroy, tick } from "svelte";
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
    ChartColumnBig,
    ChartGantt,
    Ellipsis,
    ListTree,
    LinkIcon,
    Redo2,
    ClipboardList,
    Save,
    ScrollText,
    Search,
    Trash2,
    Undo2,
    ArrowDownToLine,
  } from "@lucide/svelte";
  import AppBarButton from "../components/AppBarButton.svelte";
  import DropdownMenu from "../components/DropdownMenu.svelte";
  import LogPanel from "../components/LogPanel.svelte";
  import ReviewChangesPanel from "../components/ReviewChangesPanel.svelte";
  import AddItemDialog from "../components/AddItemDialog.svelte";
  import WorkItemExecutionTracker, {
    setWorkItemExecutionTrackerContext,
    WorkItemExecutionTrackerContext,
  } from "../components/WorkItemExecutionTracker.svelte";
  import WorkItemStatistics from "../components/WorkItemStatistics.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { ReleaseInfo } from "$lib/bindings/ReleaseInfo";
  import type { RefreshSummary } from "$lib/bindings/RefreshSummary";

  const context = setWorkItemContext(new WorkItemContext());
  setWorkItemExecutionTrackerContext(new WorkItemExecutionTrackerContext());

  type Mode = "items" | "xtracker" | "statistics";
  type StatisticsPivotField =
    | "kind"
    | "epic"
    | "workstream"
    | "assigned"
    | "status";
  type StatisticsSeriesPivotField = "none" | StatisticsPivotField;
  let mode = $state<Mode>("items");
  let statisticsRowPivotField = $state<StatisticsPivotField>("kind");
  let statisticsSeriesPivotField =
    $state<StatisticsSeriesPivotField>("none");

  const modeIcon = $derived(
    mode === "items" ? ListTree : mode === "xtracker" ? ChartGantt : ChartColumnBig
  );
  const modeText = $derived(
    mode === "items" ? "Items" : mode === "xtracker" ? "X-tracker" : "Statistics"
  );

  // Changes toolbar state
  const numChanges = $derived(Object.keys(context.data.changes.data).length);
  const canUndo = $derived(context.data.canUndo);
  const canRedo = $derived(context.data.canRedo);
  const numEpicConflicts = $derived(context.data.epicConflicts.length);

  let saveProgress = $state(0);
  let refreshSummaryMessage = $state<string | null>(null);
  let refreshSummaryTimer: ReturnType<typeof setTimeout> | null = null;
  let reviewChangesOpen = $state(false);
  let addItemDialogOpen = $state(false);
  let logPanelOpen = $state(false);
  let busy = $state(false);
  const disabled = $derived(busy || context.loadProgress > 0);

  let openDropdown = $state<"mode" | "more" | null>(null);

  async function openFind(): Promise<void> {
    await tick();
    document.dispatchEvent(new CustomEvent("ghui:open-find"));
  }

  // Update check state
  let updateInfo = $state<ReleaseInfo | null>(null);
  let updateCheckState = $state<"idle" | "checking" | "downloading">("idle");

  function showRefreshSummary({
    newItems,
    updatedItems,
  }: RefreshSummary): void {
    const parts: string[] = [];
    if (newItems > 0) {
      parts.push(`${newItems} new ${newItems === 1 ? "item" : "items"}`);
    }
    if (updatedItems > 0) {
      parts.push(
        `${updatedItems} updated ${updatedItems === 1 ? "item" : "items"}`
      );
    }

    refreshSummaryMessage =
      parts.length === 0
        ? "Refresh complete: no changes found."
        : `Refresh complete: ${parts.join(", ")}.`;

    if (refreshSummaryTimer) {
      clearTimeout(refreshSummaryTimer);
    }
    refreshSummaryTimer = setTimeout(() => {
      refreshSummaryMessage = null;
      refreshSummaryTimer = null;
    }, 5000);
  }

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
    await runBusy(async () => {
      const summary = await context.refresh();
      showRefreshSummary(summary);
    });
  }

  onDestroy(() => {
    if (refreshSummaryTimer) {
      clearTimeout(refreshSummaryTimer);
      refreshSummaryTimer = null;
    }
    refreshSummaryMessage = null;
  });

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
  {#if openDropdown !== null}
    <button
      class="fixed inset-0 z-40 cursor-default"
      aria-label="Close menu"
      tabindex="-1"
      onclick={() => (openDropdown = null)}
    ></button>
  {/if}
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
        icon={ClipboardList}
        text="Review Changes"
        disabled={!(numChanges || numEpicConflicts) || disabled}
        badge={numChanges + numEpicConflicts || undefined}
        onclick={() => {
          reviewChangesOpen = true;
          recordTelemetry({ event: "pending_changes_opened" });
        }}
      />

      <div class="w-3"></div>

      <AppBarButton
        icon={Bubbles}
        text="Sanitize"
        disabled={disabled}
        onclick={() => runBusy(() => context.sanitize())}
      />

      <div class="w-3"></div>

      <DropdownMenu
        open={openDropdown === "mode"}
        onopen={() => (openDropdown = "mode")}
        onclose={() => (openDropdown = null)}
        icon={modeIcon}
        text={modeText}
        {disabled}
        items={[
          {
            icon: ListTree,
            label: "Items",
            disabled,
            onclick: () => {
              if (mode !== "items") {
                recordTelemetry({ event: "mode_switched", to: "items" });
              }
              mode = "items";
            },
          },
          {
            icon: ChartGantt,
            label: "X-tracker",
            disabled,
            onclick: () => {
              if (mode !== "xtracker") {
                recordTelemetry({ event: "mode_switched", to: "xtracker" });
              }
              mode = "xtracker";
            },
          },
          {
            icon: ChartColumnBig,
            label: "Statistics",
            disabled,
            onclick: () => {
              if (mode !== "statistics") {
                recordTelemetry({ event: "mode_switched", to: "statistics" });
              }
              mode = "statistics";
            },
          },
        ]}
      />
      <DropdownMenu
        open={openDropdown === "more"}
        onopen={() => (openDropdown = "more")}
        onclose={() => (openDropdown = null)}
        icon={Ellipsis}
        text="More"
        items={[
          {
            icon: Undo2,
            label: "Undo",
            disabled: !canUndo || disabled,
            onclick: () => { void runBusy(() => context.undoChange()); },
          },
          {
            icon: Redo2,
            label: "Redo",
            disabled: !canRedo || disabled,
            onclick: () => { void runBusy(() => context.redoChange()); },
          },
          {
            icon: LinkIcon,
            label: "Add",
            disabled,
            onclick: () => {
              addItemDialogOpen = true;
            },
          },
          {
            icon: Trash2,
            label: "Discard",
            disabled: !numChanges || disabled,
            onclick: () => { void runBusy(() => context.deleteChanges()); },
          },
          {
            icon: Search,
            label: "Find",
            disabled: disabled || mode !== "items",
            onclick: () => { void openFind(); },
          },
          {
            icon: ArrowDownToLine,
            label: updateButtonText,
            iconClass: updateIconClass,
            disabled: updateButtonDisabled,
            onclick: () => { void onUpdateClicked(); },
          },
          {
            icon: ScrollText,
            label: "Output",
            badge: context.unreadErrorCount > 0 ? context.unreadErrorCount : undefined,
            onclick: () => {
              logPanelOpen = !logPanelOpen;
              recordTelemetry({ event: "log_panel_toggled", open: logPanelOpen });
              if (logPanelOpen) {
                context.markErrorsAsRead();
              }
            },
          },
        ]}
      />

    {/snippet}

    {#snippet trail()}
      <Pat />
    {/snippet}
  </AppBar>

  <ReviewChangesPanel bind:open={reviewChangesOpen} />
  <AddItemDialog bind:open={addItemDialogOpen} />

  <div class="flex flex-col flex-1 min-h-0 overflow-hidden">
    {#if mode === "items"}
      <WorkItemTree />
    {:else if mode === "xtracker"}
      <WorkItemExecutionTracker />
    {:else if mode === "statistics"}
      <WorkItemStatistics
        bind:rowPivotField={statisticsRowPivotField}
        bind:seriesPivotField={statisticsSeriesPivotField}
      />
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

  {#if refreshSummaryMessage}
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      class="fixed top-16 right-4 z-10 rounded-md border border-surface-400-600 bg-surface-100-900 px-3 py-2 text-sm shadow-md"
    >
      {refreshSummaryMessage}
    </div>
  {/if}
</div>

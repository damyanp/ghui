<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import {
    getWorkItemContext,
    linkTitle,
  } from "$lib/WorkItemContext.svelte";
  import { Eye, EyeOff } from "@lucide/svelte";
  import type { Change } from "$lib/bindings/Change";
  import type { Fields } from "$lib/bindings/Fields";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import type { WorkItemId } from "$lib/bindings/WorkItemId";

  type Tab = "changes" | "preview" | "conflicts";

  type Props = {
    open?: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const context = getWorkItemContext();

  // ── Pending Changes ─────────────────────────────────────────────────────────

  const changes = $derived.by(() => {
    return Object.values(context.data.changes.data).filter(Boolean) as Change[];
  });

  function getDisplayName(workItem: WorkItem | undefined): string {
    if (!workItem) return "?";
    if (workItem.resourcePath) return linkTitle(workItem);
    return workItem.title || "?";
  }

  function getWorkItemLabel(workItemId: WorkItemId): string {
    const item =
      context.data.workItems[workItemId] ??
      context.data.originalWorkItems[workItemId];
    if (!item) return workItemId;
    return getDisplayName(item);
  }

  function getWorkItemTitle(workItemId: WorkItemId): string {
    const item =
      context.data.workItems[workItemId] ??
      context.data.originalWorkItems[workItemId];
    return item?.title ?? "";
  }

  function describeChange(change: Change): string {
    switch (change.data.type) {
      case "setParent": {
        let parent = context.data.workItems[change.data.value];
        let parentDisplay = getDisplayName(parent) || "???";
        return `Set parent to '${parentDisplay}'`;
      }
      case "addToProject": {
        return "Add to project";
      }
      case "issueType": {
        let item = context.data.workItems[change.workItemId];
        if (item?.data.type === "issue") {
          if (item.data.issueType.loadState === "loaded") {
            if (item.data.issueType.value)
              return `Set issue type to ${item.data.issueType.value}`;
            return "Clear issue type";
          }
          return "Error: issue type not loaded";
        }
        return "Error: issue type change for non-issue!";
      }
      default: {
        return `Set ${change.data.type} to '${context.getFieldOption(
          change.data.type as keyof Fields,
          change.data.value
        )}'`;
      }
    }
  }

  const groupedChanges = $derived.by(() => {
    const grouped = new Map<WorkItemId, Change[]>();

    for (const change of changes) {
      const list = grouped.get(change.workItemId);
      if (list) list.push(change);
      else grouped.set(change.workItemId, [change]);
    }

    return Array.from(grouped.entries())
      .map(([workItemId, itemChanges]) => {
        const label = getWorkItemLabel(workItemId);
        const title = getWorkItemTitle(workItemId);
        const summary = itemChanges.map(describeChange).join("; ");
        return { workItemId, label, title, summary };
      })
      .sort((a, b) => a.label.localeCompare(b.label));
  });

  // ── Epic Conflicts ───────────────────────────────────────────────────────────

  const conflicts = $derived(context.data.epicConflicts);

  let selected = $state<Set<WorkItemId>>(new Set());

  // Reset selection whenever conflicts change (e.g. after staging a batch).
  $effect(() => {
    conflicts; // tracked to re-run when conflicts list changes
    selected = new Set();
  });

  const allSelected = $derived(
    conflicts.length > 0 && selected.size === conflicts.length
  );

  function getItemLabel(id: WorkItemId): string {
    const item =
      context.data.workItems[id] ?? context.data.originalWorkItems[id];
    if (!item) return id;
    if (item.resourcePath) return linkTitle(item);
    return item.title || id;
  }

  function getItemTitle(id: WorkItemId): string {
    const item =
      context.data.workItems[id] ?? context.data.originalWorkItems[id];
    return item?.title ?? "";
  }

  function getEpicName(epicId: string): string {
    return context.getFieldOption("epic", epicId) ?? epicId;
  }

  function toggleAll() {
    if (allSelected) {
      selected = new Set();
    } else {
      selected = new Set(conflicts.map((c) => c.workItemId));
    }
  }

  function toggleItem(id: WorkItemId) {
    const next = new Set(selected);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selected = next;
  }

  async function stageSelected() {
    const ids = Array.from(selected);
    await context.stageEpicOverrides(ids);
  }

  // ── Tab state ────────────────────────────────────────────────────────────────

  let activeTab = $state<Tab>("changes");

  // Switch to conflicts tab when panel is opened with conflicts present and no changes
  $effect(() => {
    if (open) {
      if (changes.length === 0 && conflicts.length > 0) {
        activeTab = "conflicts";
      } else {
        activeTab = "changes";
      }
    }
  });
</script>

<Modal
  {open}
  contentBase="card bg-surface-100-900 p-4 space-y-4 w-[800px] max-h-[80vh] flex flex-col"
  modal
  onOpenChange={(details) => {
    open = details.open;
  }}
>
  {#snippet content()}
    <header class="flex items-center justify-between gap-2">
      <div class="font-bold text-lg">Review Changes</div>
      <button type="button" class="btn p-1" onclick={() => (open = false)}>
        Close
      </button>
    </header>

    <!-- Tabs -->
    <div class="flex gap-1 border-b border-surface-300-700 pb-0">
      <button
        type="button"
        class="px-3 py-1.5 text-sm rounded-t font-medium transition-colors
          {activeTab === 'changes'
          ? 'border-b-2 border-primary-500 text-primary-500'
          : 'opacity-60 hover:opacity-100'}"
        onclick={() => (activeTab = "changes")}
      >
        Pending Changes
        {#if changes.length > 0}
          <span
            class="ml-1.5 bg-primary-500 text-white text-[0.6rem] leading-none min-w-3.5 h-3.5 inline-flex items-center justify-center rounded-full px-0.5"
          >{changes.length}</span>
        {/if}
      </button>
      <button
        type="button"
        class="px-3 py-1.5 text-sm rounded-t font-medium transition-colors
          {activeTab === 'preview'
          ? 'border-b-2 border-primary-500 text-primary-500'
          : 'opacity-60 hover:opacity-100'}"
        onclick={() => (activeTab = "preview")}
      >
        Preview
      </button>
      <button
        type="button"
        class="px-3 py-1.5 text-sm rounded-t font-medium transition-colors
          {activeTab === 'conflicts'
          ? 'border-b-2 border-primary-500 text-primary-500'
          : 'opacity-60 hover:opacity-100'}"
        onclick={() => (activeTab = "conflicts")}
      >
        Epic Conflicts
        {#if conflicts.length > 0}
          <span
            class="ml-1.5 bg-warning-500 text-white text-[0.6rem] leading-none min-w-3.5 h-3.5 inline-flex items-center justify-center rounded-full px-0.5"
          >{conflicts.length}</span>
        {/if}
      </button>
    </div>

    <!-- Tab: Pending Changes -->
    {#if activeTab === "changes"}
      {#if changes.length === 0}
        <p class="opacity-70">No pending changes.</p>
      {:else}
        <div class="overflow-y-auto flex-1 space-y-2">
          {#each groupedChanges as group (group.workItemId)}
            <div class="border rounded-2xl px-3 py-2 bg-surface-50-950">
              <div class="text-sm">
                <span class="font-semibold">{group.label}</span>
                {#if group.title}
                  <span class="opacity-80"> — {group.title}</span>
                {/if}
                <span class="opacity-70">: {group.summary}</span>
              </div>
            </div>
          {/each}
        </div>
      {/if}

    <!-- Tab: Preview -->
    {:else if activeTab === "preview"}
      <div class="flex-1 space-y-4">
        <p class="text-sm opacity-70">
          When preview is on, the item list shows how items will look after
          your pending changes are saved.
        </p>
        <button
          type="button"
          class="btn variant-filled-primary flex items-center gap-2"
          disabled={changes.length === 0}
          onclick={async () => {
            await context.setPreviewChanges(!context.previewChanges);
          }}
        >
          {#if context.previewChanges}
            <EyeOff class="w-4 h-4" />
            Turn Off Preview
          {:else}
            <Eye class="w-4 h-4" />
            Turn On Preview
          {/if}
        </button>
        {#if context.previewChanges}
          <p class="text-sm text-success-600-400">
            Preview is currently <strong>on</strong>. The item list reflects
            your pending changes.
          </p>
        {:else}
          <p class="text-sm opacity-70">
            Preview is currently <strong>off</strong>.
          </p>
        {/if}
      </div>

    <!-- Tab: Epic Conflicts -->
    {:else if activeTab === "conflicts"}
      <p class="text-sm opacity-70">
        These items already have an Epic set that differs from the one their
        parent hierarchy requires. Sanitize skipped them. Select the ones you
        want to override and click <strong>Stage Selected</strong>.
      </p>

      {#if conflicts.length === 0}
        <p class="opacity-70">No epic conflicts.</p>
      {:else}
        <div class="overflow-y-auto flex-1">
          <table class="w-full text-sm border-collapse">
            <thead>
              <tr class="text-left border-b border-surface-300-700">
                <th class="py-1 pr-2 w-8">
                  <input
                    type="checkbox"
                    checked={allSelected}
                    onchange={toggleAll}
                    title="Select all"
                  />
                </th>
                <th class="py-1 pr-3">Issue</th>
                <th class="py-1 pr-3">Current Epic</th>
                <th class="py-1">Proposed Epic</th>
              </tr>
            </thead>
            <tbody>
              {#each conflicts as conflict (conflict.workItemId)}
                {@const label = getItemLabel(conflict.workItemId)}
                {@const title = getItemTitle(conflict.workItemId)}
                {@const isChecked = selected.has(conflict.workItemId)}
                <tr
                  class="border-b border-surface-200-800 hover:bg-surface-50-950 cursor-pointer"
                  onclick={() => toggleItem(conflict.workItemId)}
                >
                  <td class="py-1.5 pr-2">
                    <input
                      type="checkbox"
                      checked={isChecked}
                      onchange={() => toggleItem(conflict.workItemId)}
                      onclick={(e) => e.stopPropagation()}
                    />
                  </td>
                  <td class="py-1.5 pr-3">
                    <span class="font-semibold">{label}</span>
                    {#if title}
                      <span class="opacity-70"> — {title}</span>
                    {/if}
                  </td>
                  <td class="py-1.5 pr-3 text-warning-600-400">
                    {getEpicName(conflict.currentEpic)}
                  </td>
                  <td class="py-1.5 text-success-600-400">
                    {getEpicName(conflict.proposedEpic)}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <footer class="flex items-center gap-3 pt-1">
          <button
            type="button"
            class="btn variant-filled-primary"
            disabled={selected.size === 0}
            onclick={stageSelected}
          >
            Stage Selected ({selected.size})
          </button>
          <button
            type="button"
            class="btn variant-ghost"
            onclick={toggleAll}
          >
            {allSelected ? "Deselect All" : "Select All"}
          </button>
        </footer>
      {/if}
    {/if}
  {/snippet}
</Modal>

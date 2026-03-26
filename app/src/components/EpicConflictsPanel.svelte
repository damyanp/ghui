<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import {
    getWorkItemContext,
    linkTitle,
  } from "$lib/WorkItemContext.svelte";
  import type { SanitizeConflict } from "$lib/bindings/SanitizeConflict";
  import type { WorkItemId } from "$lib/bindings/WorkItemId";

  type Props = {
    open?: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const context = getWorkItemContext();

  const conflicts = $derived(context.data.epicConflicts);

  let selected = $state<Set<WorkItemId>>(new Set());

  // Reset selection whenever conflicts change (e.g. after staging a batch).
  $effect(() => {
    conflicts;
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
    // Conflicts list updates automatically via DataUpdate; if now empty, close.
    if (context.data.epicConflicts.length === 0) {
      open = false;
    }
  }
</script>

<Modal
  open={open}
  contentBase="card bg-surface-100-900 p-4 space-y-4 w-[800px] max-h-[80vh] flex flex-col"
  modal
  onOpenChange={(details) => {
    open = details.open;
  }}
>
  {#snippet content()}
    <header class="flex items-center justify-between gap-2">
      <div class="font-bold text-lg">
        Epic conflicts ({conflicts.length})
      </div>
      <button type="button" class="btn p-1" onclick={() => (open = false)}>
        Close
      </button>
    </header>

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
  {/snippet}
</Modal>

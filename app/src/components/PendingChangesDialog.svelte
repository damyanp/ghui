<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import {
    getWorkItemContext,
    linkTitle,
  } from "$lib/WorkItemContext.svelte";
  import type { Change } from "$lib/bindings/Change";
  import type { Fields } from "$lib/bindings/Fields";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import type { WorkItemId } from "$lib/bindings/WorkItemId";

  type Props = {
    open?: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const context = getWorkItemContext();

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

        return {
          workItemId,
          label,
          title,
          summary,
        };
      })
      .sort((a, b) => a.label.localeCompare(b.label));
  });
</script>

<Modal
  open={open}
  contentBase="card bg-surface-100-900 p-4 space-y-4 w-[720px] max-h-[80vh] flex flex-col"
  modal
  onOpenChange={(details) => {
    open = details.open;
  }}
>
  {#snippet content()}
    <header class="flex items-center justify-between gap-2">
      <div class="font-bold text-lg">Pending changes ({changes.length})</div>
      <button type="button" class="btn p-1" onclick={() => (open = false)}>
        Close
      </button>
    </header>

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
  {/snippet}
</Modal>

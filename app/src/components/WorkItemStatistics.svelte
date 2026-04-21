<script lang="ts">
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import type { WorkItemContext } from "$lib/WorkItemContext.svelte";
  import type { Issue } from "$lib/bindings/Issue";
  import type { WorkItem } from "$lib/bindings/WorkItem";

  type PivotField = "none" | "epic" | "workstream" | "assigned" | "status";
  type LoadedField = "epic" | "status";
  type DelayLoadedField = "kind" | "workstream";
  type IssueWorkItem = WorkItem & { data: { type: "issue" } & Issue };

  type StatisticsContext = Pick<WorkItemContext, "data" | "getFieldOption">;

  type Props = {
    context?: StatisticsContext;
  };

  let { context = getWorkItemContext() }: Props = $props();
  let pivotField = $state<PivotField>("none");

  const issueItems = $derived.by(() => {
    const items: IssueWorkItem[] = [];
    for (const node of context.data.nodes) {
      if (node.data.type !== "workItem") continue;
      const item = context.data.workItems[node.id];
      if (isIssueWorkItem(item)) {
        items.push(item);
      }
    }
    return items;
  });

  const chartRows = $derived.by(() => {
    const grouped = new Map<string, Map<string, number>>();
    for (const issue of issueItems) {
      const kind = getKind(issue);
      const pivotValues = getPivotValues(issue, pivotField);
      const bucket = grouped.get(kind) ?? new Map<string, number>();
      for (const pivotValue of pivotValues) {
        bucket.set(pivotValue, (bucket.get(pivotValue) ?? 0) + 1);
      }
      grouped.set(kind, bucket);
    }

    return [...grouped.entries()]
      .map(([kind, counts]) => {
        const segments = [...counts.entries()]
          .map(([name, count]) => ({ name, count }))
          .sort((a, b) => b.count - a.count);
        const total = segments.reduce((sum, segment) => sum + segment.count, 0);
        return { kind, segments, total };
      })
      .sort((a, b) => b.total - a.total);
  });

  const maxRowTotal = $derived(
    chartRows.length > 0 ? Math.max(...chartRows.map((row) => row.total)) : 0
  );

  const pivotTotals = $derived.by(() => {
    const totals = new Map<string, number>();
    for (const row of chartRows) {
      for (const segment of row.segments) {
        totals.set(segment.name, (totals.get(segment.name) ?? 0) + segment.count);
      }
    }
    return [...totals.entries()]
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count);
  });

  function getKind(item: IssueWorkItem): string {
    return getDelayLoadFieldValueLabel(item, "kind");
  }

  function getPivotValues(item: IssueWorkItem, pivot: PivotField): string[] {
    switch (pivot) {
      case "none":
        return ["All issues"];
      case "assigned":
        return item.data.assignees.length > 0
          ? item.data.assignees
          : ["(unassigned)"];
      case "epic":
      case "status":
        return [getFieldValueLabel(item, pivot)];
      case "workstream":
        return [getDelayLoadFieldValueLabel(item, pivot)];
    }
  }

  function getFieldValueLabel(item: IssueWorkItem, fieldName: LoadedField): string {
    const fieldValue = item.projectItem[fieldName];
    if (typeof fieldValue === "object") return "(not loaded)";
    return context.getFieldOption(fieldName, fieldValue) ?? "(none)";
  }

  function getDelayLoadFieldValueLabel(
    item: IssueWorkItem,
    fieldName: DelayLoadedField
  ): string {
    const fieldValue = item.projectItem[fieldName];
    if (typeof fieldValue !== "object") {
      return context.getFieldOption(fieldName, fieldValue) ?? "(none)";
    }
    if (fieldValue.loadState !== "loaded") return "(not loaded)";
    return context.getFieldOption(fieldName, fieldValue.value) ?? "(none)";
  }

  function getSegmentColor(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = (hash * 31 + name.charCodeAt(i)) % 360;
    }
    return `hsl(${hash} 65% 55%)`;
  }

  function isIssueWorkItem(item: WorkItem | undefined): item is IssueWorkItem {
    return item?.data.type === "issue";
  }
</script>

<div class="overflow-y-auto flex-1 p-3">
  <div class="mb-4 flex items-center gap-3">
    <label for="pivot-field" class="text-sm font-semibold">Pivot</label>
    <select id="pivot-field" class="select variant-form" bind:value={pivotField}>
      <option value="none">None</option>
      <option value="epic">Epic</option>
      <option value="workstream">Workstream</option>
      <option value="assigned">Assigned</option>
      <option value="status">Status</option>
    </select>
    <div class="text-sm text-surface-700-300">
      {issueItems.length} filtered issues
    </div>
  </div>

  {#if chartRows.length === 0}
    <div class="text-surface-700-300">No filtered issues to chart.</div>
  {:else}
    <div class="flex flex-col gap-2">
      {#each chartRows as row}
        <div class="grid grid-cols-[12rem_1fr_3rem] items-center gap-2">
          <div class="truncate text-sm" title={row.kind}>{row.kind}</div>
          <div class="h-5 rounded bg-surface-200-800 overflow-hidden flex">
            {#each row.segments as segment}
              <div
                class="h-full min-w-[2px]"
                title={`${segment.name}: ${segment.count}`}
                style={`width: ${(segment.count / row.total) * 100}%; background-color: ${getSegmentColor(segment.name)}; opacity: ${maxRowTotal === 0
                  ? 1
                  : 0.35 + (0.65 * row.total) / maxRowTotal}`}
              ></div>
            {/each}
          </div>
          <div class="text-right text-sm tabular-nums">{row.total}</div>
        </div>
      {/each}
    </div>
  {/if}

  {#if pivotField !== "none" && pivotTotals.length > 0}
    <div class="mt-5">
      <div class="text-sm font-semibold mb-2">Pivot legend</div>
      <div class="flex flex-wrap gap-3">
        {#each pivotTotals as pivot}
          <div class="flex items-center gap-2 text-sm">
            <span
              class="inline-block h-3 w-3 rounded"
              style={`background-color: ${getSegmentColor(pivot.name)}`}
            ></span>
            <span class="truncate max-w-[20rem]" title={pivot.name}>
              {pivot.name}
            </span>
            <span class="tabular-nums text-surface-700-300">{pivot.count}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

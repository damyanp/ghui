<script lang="ts">
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import type { WorkItemContext } from "$lib/WorkItemContext.svelte";
  import type { Issue } from "$lib/bindings/Issue";
  import type { WorkItem } from "$lib/bindings/WorkItem";

  type PivotField = "kind" | "epic" | "workstream" | "assigned" | "status";
  type SeriesPivotField = "none" | PivotField;
  type LoadedField = "epic" | "status";
  type DelayLoadedField = "kind" | "workstream";
  type IssueWorkItem = WorkItem & { data: { type: "issue" } & Issue };
  const SEGMENT_COLOR_CLASSES = [
    "bg-primary-500",
    "bg-secondary-500",
    "bg-tertiary-500",
    "bg-success-500",
    "bg-warning-500",
    "bg-error-500",
    "bg-primary-700",
    "bg-secondary-700",
    "bg-tertiary-700",
    "bg-success-700",
    "bg-warning-700",
    "bg-error-700",
  ];
  const MIN_PROGRESS_BAR_WIDTH_PERCENT = 4;
  const MAX_LOAD_ATTEMPTS_PER_ISSUE = 3;
  const MAX_IN_FLIGHT_LOAD_REQUESTS = 8;

  type StatisticsContext = Pick<
    WorkItemContext,
    "data" | "getFieldOption" | "updateWorkItem" | "loadProgress"
  >;

  type Props = {
    context?: StatisticsContext;
    rowPivotField?: PivotField;
    seriesPivotField?: SeriesPivotField;
  };

  let {
    context = getWorkItemContext(),
    rowPivotField = $bindable<PivotField>("kind"),
    seriesPivotField = $bindable<SeriesPivotField>("none"),
  }: Props = $props();

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
      const rowValues = getPivotValues(issue, rowPivotField);
      const seriesValues =
        seriesPivotField === "none"
          ? ["All issues"]
          : getPivotValues(issue, seriesPivotField);

      for (const rowValue of rowValues) {
        let bucket = grouped.get(rowValue);
        if (!bucket) {
          bucket = new Map<string, number>();
          grouped.set(rowValue, bucket);
        }

        for (const seriesValue of seriesValues) {
          bucket.set(seriesValue, (bucket.get(seriesValue) ?? 0) + 1);
        }
      }
    }

    return [...grouped.entries()]
      .map(([rowName, counts]) => {
        const segments = [...counts.entries()]
          .map(([name, count]) => ({ name, count }))
          .sort((a, b) => b.count - a.count);
        const total = segments.reduce((sum, segment) => sum + segment.count, 0);
        return { rowName, segments, total };
      })
      .sort((a, b) => b.total - a.total);
  });

  let isIssueLoadFailed = $state<Record<string, true>>({});
  let loadAttemptsByIssue = $state<Record<string, number>>({});

  const pendingIssueIds = $derived.by(() =>
    issueItems
      .filter(
        (issue) =>
          !isIssueLoadedForStatistics(issue) && !isIssueLoadFailed[issue.id]
      )
      .map((issue) => issue.id)
  );

  const statisticsLoadProgress = $derived(
    issueItems.length === 0
      ? 0
      : (issueItems.length - pendingIssueIds.length) / issueItems.length
  );
  const statisticsLoadProgressPercent = $derived(
    Math.round(statisticsLoadProgress * 100)
  );

  const maxRowTotal = $derived(
    chartRows.length > 0 ? Math.max(...chartRows.map((row) => row.total)) : 0
  );

  const seriesTotals = $derived.by(() => {
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

  const requestedLoadIds = new Set<string>();
  function markIssueLoadFailed(issueId: string): void {
    if (!isIssueLoadFailed[issueId]) {
      isIssueLoadFailed = { ...isIssueLoadFailed, [issueId]: true };
    }
  }

  $effect(() => {
    const pendingSet = new Set(pendingIssueIds);
    const issueIdSet = new Set(issueItems.map((issue) => issue.id));

    let shouldUpdateFailedIssues = false;
    const nextFailedIssues: Record<string, true> = {};
    for (const issueId of Object.keys(isIssueLoadFailed)) {
      if (issueIdSet.has(issueId)) {
        nextFailedIssues[issueId] = true;
      } else {
        shouldUpdateFailedIssues = true;
      }
    }
    if (shouldUpdateFailedIssues) {
      isIssueLoadFailed = nextFailedIssues;
    }

    let shouldUpdateAttempts = false;
    const nextLoadAttempts: Record<string, number> = {};
    for (const [issueId, attempts] of Object.entries(loadAttemptsByIssue)) {
      if (issueIdSet.has(issueId)) {
        nextLoadAttempts[issueId] = attempts;
      } else {
        shouldUpdateAttempts = true;
      }
    }
    if (shouldUpdateAttempts) {
      loadAttemptsByIssue = nextLoadAttempts;
    }

    for (const issueId of [...requestedLoadIds]) {
      if (!issueIdSet.has(issueId)) requestedLoadIds.delete(issueId);
    }

    for (const issueId of pendingIssueIds) {
      if (requestedLoadIds.size >= MAX_IN_FLIGHT_LOAD_REQUESTS) break;
      if (requestedLoadIds.has(issueId)) continue;
      const attempts = loadAttemptsByIssue[issueId] ?? 0;
      if (attempts >= MAX_LOAD_ATTEMPTS_PER_ISSUE) {
        markIssueLoadFailed(issueId);
        continue;
      }

      loadAttemptsByIssue = { ...loadAttemptsByIssue, [issueId]: attempts + 1 };
      requestedLoadIds.add(issueId);
      context.updateWorkItem(issueId).catch(() => {
        console.warn(`Failed to load statistics data for issue ${issueId}`);
        requestedLoadIds.delete(issueId);
      });
    }

    for (const issueId of [...requestedLoadIds]) {
      if (!pendingSet.has(issueId)) requestedLoadIds.delete(issueId);
    }
  });

  function getPivotValues(item: IssueWorkItem, pivot: PivotField): string[] {
    switch (pivot) {
      case "kind":
      case "workstream":
        return [getDelayLoadFieldValueLabel(item, pivot)];
      case "epic":
      case "status":
        return [getLoadedFieldValueLabel(item, pivot)];
      case "assigned":
        return item.data.assignees.length > 0
          ? item.data.assignees
          : ["(unassigned)"];
      default: {
        const _exhaustive: never = pivot;
        return [_exhaustive];
      }
    }
  }

  function getLoadedFieldValueLabel(
    item: IssueWorkItem,
    fieldName: LoadedField
  ): string {
    const fieldValue = item.projectItem[fieldName];
    if (typeof fieldValue === "object") return "(none)";
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
    if (fieldValue.loadState !== "loaded") return "(none)";
    return context.getFieldOption(fieldName, fieldValue.value) ?? "(none)";
  }

  function isIssueLoadedForStatistics(item: IssueWorkItem): boolean {
    return (
      isLoadedFieldValueLoaded(item, "epic") &&
      isLoadedFieldValueLoaded(item, "status") &&
      isDelayLoadedFieldValueLoaded(item, "kind") &&
      isDelayLoadedFieldValueLoaded(item, "workstream")
    );
  }

  function isLoadedFieldValueLoaded(
    item: IssueWorkItem,
    fieldName: LoadedField
  ): boolean {
    return typeof item.projectItem[fieldName] !== "object";
  }

  function isDelayLoadedFieldValueLoaded(
    item: IssueWorkItem,
    fieldName: DelayLoadedField
  ): boolean {
    const fieldValue = item.projectItem[fieldName];
    return typeof fieldValue !== "object" || fieldValue.loadState === "loaded";
  }

  function getSegmentColor(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = (hash * 31 + name.charCodeAt(i)) >>> 0;
    }
    return SEGMENT_COLOR_CLASSES[hash % SEGMENT_COLOR_CLASSES.length];
  }

  function getSegmentWidth(count: number): number {
    if (maxRowTotal === 0) return 0;
    return (count / maxRowTotal) * 100;
  }

  function isIssueWorkItem(item: WorkItem | undefined): item is IssueWorkItem {
    return item !== undefined && item.data.type === "issue";
  }
</script>

<div class="overflow-y-auto flex-1 p-3">
  <div class="mb-4 flex flex-wrap items-center gap-3">
    <label for="row-pivot-field" class="text-sm font-semibold">Rows</label>
    <select
      id="row-pivot-field"
      class="select variant-form"
      value={rowPivotField}
      onchange={(e) => {
        const next = (e.currentTarget as HTMLSelectElement).value as PivotField;
        rowPivotField = next;
        if (seriesPivotField === next) {
          seriesPivotField = "none";
        }
      }}
    >
      <option value="kind">Kind</option>
      <option value="epic">Epic</option>
      <option value="workstream">Workstream</option>
      <option value="assigned">Assigned</option>
      <option value="status">Status</option>
    </select>

    <label for="series-pivot-field" class="text-sm font-semibold">Series</label>
    <select
      id="series-pivot-field"
      class="select variant-form"
      bind:value={seriesPivotField}
    >
      <option value="none">None</option>
      {#if rowPivotField !== "kind"}
        <option value="kind">Kind</option>
      {/if}
      {#if rowPivotField !== "epic"}
        <option value="epic">Epic</option>
      {/if}
      {#if rowPivotField !== "workstream"}
        <option value="workstream">Workstream</option>
      {/if}
      {#if rowPivotField !== "assigned"}
        <option value="assigned">Assigned</option>
      {/if}
      {#if rowPivotField !== "status"}
        <option value="status">Status</option>
      {/if}
    </select>

    <div class="text-sm text-surface-700-300">
      {issueItems.length} filtered issues
    </div>
  </div>

  {#if pendingIssueIds.length > 0}
    <div class="mb-4 rounded border border-surface-300-700 p-3">
      <div class="mb-2 flex items-center justify-between text-sm">
        <span>Loading issue field data for statistics…</span>
        <span class="tabular-nums">{statisticsLoadProgressPercent}%</span>
      </div>
      <div class="h-2 overflow-hidden rounded bg-surface-200-800">
        <div
          class="h-full bg-primary-500 transition-[width] duration-200"
          style={`width: ${Math.max(MIN_PROGRESS_BAR_WIDTH_PERCENT, statisticsLoadProgress * 100)}%`}
        ></div>
      </div>
    </div>
  {/if}

  {#if issueItems.length === 0}
    <div class="text-surface-700-300">No filtered issues to chart.</div>
  {:else if chartRows.length === 0}
    <div class="text-surface-700-300">No filtered issues to chart.</div>
  {:else}
    <div class="mb-2 text-xs text-surface-700-300">
      Bar width scaled to max row total ({maxRowTotal}).
    </div>
    <div class="flex flex-col gap-2">
      {#each chartRows as row}
        <div class="grid grid-cols-[14rem_1fr_3rem] items-center gap-2">
          <div class="truncate text-sm" title={row.rowName}>{row.rowName}</div>
          <div class="h-5 rounded bg-surface-200-800 overflow-hidden flex">
            {#each row.segments as segment}
              <div
                class={`h-full min-w-[2px] ${getSegmentColor(segment.name)}`}
                title={`${segment.name}: ${segment.count}`}
                style={`width: ${getSegmentWidth(segment.count)}%`}
              ></div>
            {/each}
          </div>
          <div class="text-right text-sm tabular-nums">{row.total}</div>
        </div>
      {/each}
    </div>
  {/if}

  {#if seriesTotals.length > 0}
    <div class="mt-5">
      <div class="text-sm font-semibold mb-2">Series legend</div>
      <div class="flex flex-wrap gap-3">
        {#each seriesTotals as series}
          <div class="flex items-center gap-2 text-sm">
            <span
              class={`inline-block h-3 w-3 rounded ${getSegmentColor(series.name)}`}
            ></span>
            <span class="truncate max-w-[20rem]" title={series.name}>
              {series.name}
            </span>
            <span class="tabular-nums text-surface-700-300">{series.count}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

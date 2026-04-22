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
  const STALL_THRESHOLD_MS = 10_000;
  const MAX_RECENT_PROGRESS_EVENTS = 10;
  const MAX_DEBUG_PENDING_IDS = 20;

  type StatisticsContext = Pick<
    WorkItemContext,
    "data" | "getFieldOption" | "updateWorkItem" | "loadProgress" | "loadAllWorkItems"
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

  let isLoadingAll = $state(false);
  let loadError = $state<string | null>(null);
  let lastProgressEvents = $state<
    Array<{ timestamp: number; pendingCount: number; loadedCount: number }>
  >([]);
  let lastProgressAt = $state<number | null>(null);
  let now = $state(Date.now());

  const pendingIssueIds = $derived.by(() =>
    issueItems
      .filter((issue) => !isIssueLoadedForStatistics(issue))
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

  // Record a progress event whenever the pending count changes.
  $effect(() => {
    const loadedCount = issueItems.length - pendingIssueIds.length;
    const pendingCount = pendingIssueIds.length;
    const last = lastProgressEvents[lastProgressEvents.length - 1];
    if (
      last === undefined ||
      last.pendingCount !== pendingCount ||
      last.loadedCount !== loadedCount
    ) {
      const event = { timestamp: Date.now(), pendingCount, loadedCount };
      lastProgressEvents = [
        ...lastProgressEvents.slice(-(MAX_RECENT_PROGRESS_EVENTS - 1)),
        event,
      ];
      lastProgressAt = event.timestamp;
    }
  });

  // Kick off a single batch load of every missing item on mount (and whenever
  // the backend reports new items that need loading). `loadAllWorkItems()`
  // awaits all chunks in the backend, so its promise actually represents
  // completion — unlike `updateWorkItem()` which was fire-and-forget.
  //
  // We record which issue IDs were attempted and refuse to re-fire while the
  // pending set is unchanged. The key is set *before* the call (not after
  // success) so a persistent failure doesn't immediately re-trigger the
  // request when `isLoadingAll` flips back to false in `finally`. On failure
  // we apply an exponential timestamp-based backoff (capped) before clearing
  // the key, so transient errors are retried but a broken backend (offline,
  // missing PAT) doesn't get hammered. The key uses sorted IDs so a pure
  // reorder of `pendingIssueIds` doesn't trigger a redundant call. A change
  // in the pending set (new IDs appearing) bypasses the backoff and retries
  // immediately, which is what we want.
  const FAILURE_BACKOFF_BASE_MS = 5_000;
  const FAILURE_BACKOFF_MAX_MS = 60_000;
  let lastAttemptedPendingKey = $state<string | null>(null);
  let failureRetryAt = $state<number | null>(null);
  let consecutiveFailures = $state(0);

  $effect(() => {
    if (isLoadingAll) return;
    if (pendingIssueIds.length === 0) return;
    if (context.loadAllWorkItems === undefined) return;

    const pendingKey = [...pendingIssueIds].sort().join(",");
    if (pendingKey === lastAttemptedPendingKey) {
      // Same pending set as last attempt. If we're in a failure-backoff
      // window, schedule a wake-up to retry once it expires.
      if (failureRetryAt !== null) {
        const delay = failureRetryAt - Date.now();
        if (delay > 0) {
          const handle = setTimeout(() => {
            lastAttemptedPendingKey = null;
            failureRetryAt = null;
          }, delay);
          return () => clearTimeout(handle);
        }
        // Backoff has already expired — clear and let the next tick retry.
        lastAttemptedPendingKey = null;
        failureRetryAt = null;
      }
      return;
    }

    console.debug(
      `[WorkItemStatistics] loadAllWorkItems start; ${pendingIssueIds.length} pending issue(s)`
    );
    isLoadingAll = true;
    loadError = null;
    // Set the attempted key eagerly so a synchronous failure doesn't loop.
    lastAttemptedPendingKey = pendingKey;
    context
      .loadAllWorkItems()
      .then(() => {
        consecutiveFailures = 0;
        failureRetryAt = null;
      })
      .catch((e: unknown) => {
        loadError = e instanceof Error ? e.message : String(e);
        console.warn("[WorkItemStatistics] loadAllWorkItems failed", e);
        consecutiveFailures += 1;
        const backoff = Math.min(
          FAILURE_BACKOFF_BASE_MS * 2 ** (consecutiveFailures - 1),
          FAILURE_BACKOFF_MAX_MS
        );
        failureRetryAt = Date.now() + backoff;
      })
      .finally(() => {
        isLoadingAll = false;
        console.debug("[WorkItemStatistics] loadAllWorkItems finished");
      });
  });

  // Tick `now` once a second while we're still waiting, so the stall detector
  // re-evaluates without needing a data update.
  $effect(() => {
    if (pendingIssueIds.length === 0) return;
    const handle = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(handle);
  });

  const elapsedSinceLastProgressMs = $derived(
    lastProgressAt === null ? 0 : now - lastProgressAt
  );
  const isStalled = $derived(
    pendingIssueIds.length > 0 &&
      lastProgressAt !== null &&
      elapsedSinceLastProgressMs > STALL_THRESHOLD_MS
  );

  // Debug panel is opt-in via `?debug=1` on the URL.
  const debugEnabled = $derived.by(() => {
    if (typeof window === "undefined") return false;
    try {
      return new URLSearchParams(window.location.search).get("debug") === "1";
    } catch {
      return false;
    }
  });

  // Per-field pending breakdown: how many pending issues are missing each
  // individual field. Helps identify a field that never resolves.
  const pendingByField = $derived.by(() => {
    const counts = { epic: 0, status: 0, kind: 0, workstream: 0 };
    for (const issue of issueItems) {
      if (!isLoadedFieldValueLoaded(issue, "epic")) counts.epic += 1;
      if (!isLoadedFieldValueLoaded(issue, "status")) counts.status += 1;
      if (!isDelayLoadedFieldValueLoaded(issue, "kind")) counts.kind += 1;
      if (!isDelayLoadedFieldValueLoaded(issue, "workstream"))
        counts.workstream += 1;
    }
    return counts;
  });

  const debugPendingIds = $derived(pendingIssueIds.slice(0, MAX_DEBUG_PENDING_IDS));

  function buildDiagnostics() {
    return {
      generatedAt: new Date().toISOString(),
      totals: {
        issues: issueItems.length,
        loaded: issueItems.length - pendingIssueIds.length,
        pending: pendingIssueIds.length,
      },
      progressPercent: statisticsLoadProgressPercent,
      isLoadingAll,
      loadError,
      isStalled,
      elapsedSinceLastProgressMs,
      pendingByField,
      pendingIssueIds: debugPendingIds.map((id) => ({
        id,
        title: context.data.workItems[id]?.title ?? null,
      })),
      recentProgressEvents: lastProgressEvents.map((e) => ({
        at: new Date(e.timestamp).toISOString(),
        loaded: e.loadedCount,
        pending: e.pendingCount,
      })),
    };
  }

  async function copyDiagnostics() {
    const json = JSON.stringify(buildDiagnostics(), null, 2);
    try {
      await navigator.clipboard.writeText(json);
    } catch (e) {
      console.warn("[WorkItemStatistics] copy diagnostics failed", e);
      console.info("[WorkItemStatistics] diagnostics payload:\n" + json);
    }
  }

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
    // `null` is a valid loaded value (the field is intentionally unset). It's
    // only `{ loadState: ... }` shapes that mean "not loaded yet". Note that
    // `typeof null === "object"` in JS, so we have to special-case `null`
    // before the typeof check.
    const value = item.projectItem[fieldName];
    return value === null || typeof value !== "object";
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
  <div class="mb-4 flex flex-nowrap items-center gap-2 text-sm">
    <label for="row-pivot-field" class="font-semibold">Rows</label>
    <select
      id="row-pivot-field"
      class="select variant-form text-sm py-1 px-2 w-auto"
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

    <label for="series-pivot-field" class="font-semibold">Series</label>
    <select
      id="series-pivot-field"
      class="select variant-form text-sm py-1 px-2 w-auto"
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

    <div class="text-surface-700-300 ml-2">
      {issueItems.length} filtered issues
    </div>
  </div>

  {#if pendingIssueIds.length > 0}
    <div class="mb-4 rounded border border-surface-300-700 p-3">
      <div class="mb-2 flex items-center justify-between text-sm">
        <span>
          {#if isStalled}
            Stalled waiting on {pendingIssueIds.length} issue{pendingIssueIds.length ===
            1
              ? ""
              : "s"} (no progress in {Math.round(elapsedSinceLastProgressMs / 1000)}s)
          {:else}
            Loading issue field data for statistics…
            ({issueItems.length - pendingIssueIds.length}/{issueItems.length})
          {/if}
        </span>
        <span class="tabular-nums">{statisticsLoadProgressPercent}%</span>
      </div>
      <div class="h-2 overflow-hidden rounded bg-surface-200-800">
        <div
          class={`h-full transition-[width] duration-200 ${isStalled ? "bg-warning-500" : "bg-primary-500"}`}
          style={`width: ${Math.max(MIN_PROGRESS_BAR_WIDTH_PERCENT, statisticsLoadProgress * 100)}%`}
        ></div>
      </div>
      {#if loadError}
        <div class="mt-2 text-xs text-error-500">
          Load error: {loadError}
        </div>
      {/if}
      {#if isStalled || debugEnabled}
        <button
          type="button"
          class="mt-2 text-xs underline text-primary-700-300"
          onclick={copyDiagnostics}
        >
          Copy diagnostics JSON
        </button>
      {/if}
    </div>
  {/if}

  {#if debugEnabled && (pendingIssueIds.length > 0 || lastProgressEvents.length > 0)}
    <div
      class="mb-4 rounded border border-surface-300-700 bg-surface-100-900 p-3 text-xs"
    >
      <div class="mb-2 font-semibold">Statistics loader debug</div>
      <div class="grid grid-cols-2 gap-x-4 gap-y-1">
        <div>Total issues</div>
        <div class="tabular-nums">{issueItems.length}</div>
        <div>Loaded</div>
        <div class="tabular-nums">
          {issueItems.length - pendingIssueIds.length}
        </div>
        <div>Pending</div>
        <div class="tabular-nums">{pendingIssueIds.length}</div>
        <div>Loader in flight</div>
        <div>{isLoadingAll ? "yes" : "no"}</div>
        <div>Last progress</div>
        <div>
          {lastProgressAt === null
            ? "(none)"
            : `${Math.round(elapsedSinceLastProgressMs / 1000)}s ago`}
        </div>
        <div>Stalled</div>
        <div>{isStalled ? "yes" : "no"}</div>
        <div>Missing epic</div>
        <div class="tabular-nums">{pendingByField.epic}</div>
        <div>Missing status</div>
        <div class="tabular-nums">{pendingByField.status}</div>
        <div>Missing kind</div>
        <div class="tabular-nums">{pendingByField.kind}</div>
        <div>Missing workstream</div>
        <div class="tabular-nums">{pendingByField.workstream}</div>
      </div>

      {#if debugPendingIds.length > 0}
        <div class="mt-3 font-semibold">
          First {debugPendingIds.length} pending issue(s)
        </div>
        <ul class="mt-1 list-disc pl-5 max-h-40 overflow-y-auto">
          {#each debugPendingIds as id}
            <li class="truncate">
              <span class="font-mono">{id}</span>
              {#if context.data.workItems[id]?.title}
                — {context.data.workItems[id]?.title}
              {/if}
            </li>
          {/each}
        </ul>
      {/if}

      {#if lastProgressEvents.length > 0}
        <div class="mt-3 font-semibold">Recent progress events</div>
        <ul class="mt-1 list-disc pl-5 max-h-40 overflow-y-auto">
          {#each [...lastProgressEvents].reverse() as event}
            <li>
              <span class="tabular-nums"
                >{new Date(event.timestamp)
                  .toISOString()
                  .substring(11, 19)}</span
              >
              — loaded {event.loadedCount}, pending {event.pendingCount}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if issueItems.length === 0}
    <div class="text-surface-700-300">No filtered issues to chart.</div>
  {:else if chartRows.length === 0}
    <div class="text-surface-700-300">No filtered issues to chart.</div>
  {:else}
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

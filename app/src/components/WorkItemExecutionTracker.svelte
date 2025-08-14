<script lang="ts" module>
  import {
    ExecutionTrackerContext,
    setExecutionTrackerContext,
  } from "./ExecutionTrackerContext.svelte";
  import { getContext, setContext } from "svelte";

  const key = Symbol("WorkItemExecutionTrackerContext");

  export function setWorkItemExecutionTrackerContext(
    c: WorkItemExecutionTrackerContext
  ) {
    setContext(key, c);
    setExecutionTrackerContext(c.executionTrackerContext);
    return c;
  }

  export function getWorkItemExecutionTrackerContext() {
    return getContext(key) as WorkItemExecutionTrackerContext;
  }

  export class WorkItemExecutionTrackerContext {
    executionTrackerContext = new ExecutionTrackerContext();
    hiddenEpics = new SvelteSet<string>();
  }
</script>

<script lang="ts">
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import {
    getWorkItemContext,
    linkHRef,
    linkTitle,
  } from "$lib/WorkItemContext.svelte";
  import dayjs from "dayjs";
  import ExecutionTracker, {
    type Bar,
    type BarState,
    type Epic,
    type Data as ExecutionTrackerData,
    type Iteration,
    type Row,
    type Scenario,
  } from "./ExecutionTracker.svelte";
  import WorkItemExtraDataEditor from "./WorkItemExtraDataEditor.svelte";
  import type { MenuItem } from "./TreeTableContextMenu.svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { Portal } from "bits-ui";

  let context = getWorkItemContext();

  let hiddenEpics = getWorkItemExecutionTrackerContext().hiddenEpics;

  const startDate = dayjs().subtract(3, "months").format("YYYY-MM-DD");

  type Payload = WorkItem;

  const epics: Epic<Payload>[] = $derived.by(() => {
    return Object.values(context.data.fields.epic.options)
      .filter((epicFieldOption) => !hiddenEpics.has(epicFieldOption.value))
      .map((epicFieldOption) => {
        return {
          name: epicFieldOption.value,
          targetDate: "TBD",
          scenarios: getScenarios(epicFieldOption.id),
        };
      });
  });

  const scenarioKindId = $derived(
    context.data.fields.kind.options.find((o) => o.value === "Scenario")?.id
  );

  const deliverableKindId = $derived(
    context.data.fields.kind.options.find((o) => o.value === "Deliverable")?.id
  );

  function getStatusId(name: string): string | undefined {
    return context.data.fields.status.options.find((o) => o.value == name)?.id;
  }

  const closedStatusId = $derived(getStatusId("Closed"));

  const statusOrdering = ["Active", "Ready", "Planning", undefined, "Closed"];
  const statusOrder = $derived.by(() => {
    return new Map(
      statusOrdering.map((name, index) => {
        return [name && getStatusId(name), index];
      })
    );
  });

  const defaultStart = startDate;
  const defaultEnd = dayjs().add(3, "month").format("YYYY-MM-DD");

  function getScenarios(epicId: FieldOptionId): Scenario<Payload>[] {
    const scenarios: Scenario<Payload>[] = Object.values(context.data.workItems)
      .filter((workItem): workItem is WorkItem => {
        if (!workItem) return false;

        return (
          workItem.projectItem.epic === epicId &&
          workItem.projectItem.kind.loadState === "loaded" &&
          workItem.projectItem.kind.value === scenarioKindId
        );
      })
      .map((scenario): Scenario<Payload> => {
        const isClosed = scenario.projectItem.status === closedStatusId;

        const extraData = context.getWorkItemExtraData(scenario.id);

        let rows: Row<WorkItem>[] = [];

        if (extraData.bars) {
          rows.push(getBars(extraData, scenario));
        }

        rows = [...rows, ...getRows(scenario)];

        if (extraData.burnDown) {
          let start = dayjs().subtract(1, "week");
          let end = dayjs().add(1, "week");

          if (extraData.start) start = dayjs(extraData.start);
          else if (extraData.burnDown === "noDates") {
            start = dayjs().add(1, "week");
            end = dayjs(defaultEnd);
          }

          rows.push({
            bars: [
              {
                label: getBurndownLabel(scenario),
                state: extraData.burnDown,
                start: start.format("YYYY-MM-DD"),
                end: end.format("YYYY-MM-DD"),
              },
            ],
          });
        }

        if (isClosed) rows = collapseRows(rows);

        if (rows.length === 0) {
          rows.push({ bars: [] });
        }

        return {
          name: cleanUpTitle(scenario.title),
          rows,
          extraClasses: isClosed ? ["text-gray-500"] : undefined,
          getMenuItems: () => getStandardMenuItems(scenario, []),
          data: scenario,
        };
      })
      .sort((a, b) => getScenarioStartDate(a) - getScenarioStartDate(b))
      .sort((a, b) => {
        const aIsClosed = a.data!.projectItem.status === closedStatusId;
        const bIsClosed = b.data!.projectItem.status === closedStatusId;
        return (
          (statusOrder.get(a.data!.projectItem.status || undefined) || 0) -
          (statusOrder.get(b.data!.projectItem.status || undefined) || 0)
        );
      });

    if (scenarios.length === 0) return [{ name: "TBD", rows: [{ bars: [] }] }];
    else return scenarios;
  }

  function getTitleMenuItem(workItem: WorkItem): MenuItem {
    return {
      type: "text",
      title: workItem.title,
    };
  }

  function getOpenMenuItem(workItem: WorkItem): MenuItem {
    return {
      type: "link",
      title: linkTitle(workItem),
      href: linkHRef(workItem),
    };
  }

  function getEditMenuItem(workItem: WorkItem): MenuItem {
    return {
      type: "action",
      title: "Edit...",
      action: () => {
        editorWorkItem = workItem;
        editorOpen = true;
      },
    };
  }

  function getScenarioStartDate(scenario: Scenario<Payload>) {
    if (scenario.rows.length === 0) return dayjs().unix();

    return scenario.rows
      .map((row) =>
        row.bars
          .map((bar) => dayjs(bar.start).unix())
          .reduce((a, b) => Math.min(a, b), Number.MAX_VALUE)
      )
      .reduce((a, b) => Math.min(a, b), Number.MAX_VALUE);
  }

  function getRows(scenario: WorkItem): Row<Payload>[] {
    const rows = getDeliverables()
      .map(buildRowFromDeliverable)
      .map(addStandardMenuItems);

    return rows;

    function getDeliverables(): WorkItem[] {
      if (scenario.data.type !== "issue") return [];

      return scenario.data.subIssues
        .map((id) => {
          return context.data.workItems[id];
        })
        .filter((i): i is WorkItem => {
          if (!i) return false;

          return (
            i.projectItem.kind.loadState === "loaded" &&
            i.projectItem.kind.value === deliverableKindId
          );
        });
    }

    function buildRowFromDeliverable(deliverable: WorkItem): Row<Payload> {
      const extraData = context.getWorkItemExtraData(deliverable.id);

      // If there are bars explicitly provided these override everything
      if (extraData) {
        if (extraData.bars) {
          return getBars(extraData, deliverable);
        }
        if (extraData.split) {
          let bars: Partial<Bar<Payload>>[] = [];
          let previousBar: Partial<Bar<Payload>> | undefined = undefined;

          for (const entry of <Partial<Bar<Payload>>[]>extraData.split) {
            let newBar = $state.snapshot(entry);
            if (!newBar.start) {
              newBar.start = previousBar?.end;
              if (!newBar.start) newBar.start = defaultStart;
            }

            if (previousBar && !previousBar.end) previousBar.end = newBar.start;

            bars.push(<Bar<Payload>>{
              ...newBar,
              data: deliverable,
            });
            previousBar = bars[bars.length - 1];
          }

          if (bars.length > 0 && !bars[bars.length - 1].end) {
            bars[bars.length - 1].end = getProjectedEnd(deliverable);
          }
          return { bars: <Bar<Payload>[]>bars };
        }
      }

      let end = getProjectedEnd(deliverable);
      let noDates = end === undefined;
      const status = deliverable.projectItem.status;

      let start: string | undefined = extraData.start;
      let estimate: number = extraData.estimate;

      if (!start) {
        if (end) {
          if (estimate)
            start = dayjs(end).subtract(estimate, "days").format("YYYY-MM-DD");
          else start = dayjs(end).subtract(2, "weeks").format("YYYY-MM-DD");
        } else {
          start = dayjs().add(1, "week").format("YYYY-MM-DD");
        }
      }

      if (!end) {
        if (estimate) {
          end = dayjs(start).add(estimate, "days").format("YYYY-MM-DD");
        } else {
          end = defaultEnd;
        }
      }

      let state: BarState =
        status === closedStatusId
          ? "completed"
          : noDates
            ? "noDates"
            : dayjs(end) < dayjs()
              ? "offTrack"
              : dayjs(start) > dayjs()
                ? "notStarted"
                : "onTrack";

      let label = cleanUpTitle(deliverable.title);
      if (extraData.burnDown) {
        label = `${label} (${getBurndownLabel(deliverable)})`;
        state = extraData.burnDown;
      }

      return {
        bars: [
          {
            state,
            label,
            start,
            end,
            data: deliverable,
          },
        ],
      };
    }
  }

  function getBars(extraData: any, workItem: WorkItem) {
    const bars = <Bar<Payload>[]>extraData.bars;
    return {
      bars: bars.map((bar) => {
        return {
          ...bar,
          data: workItem,
        };
      }),
    };
  }

  function addStandardMenuItems(row: Row<Payload>): Row<Payload> {
    return {
      ...row,
      bars: row.bars.map((bar) => {
        return {
          ...bar,
          getMenuItems: () => {
            if (bar.data) {
              return getStandardMenuItems(bar.data, bar.getMenuItems?.());
            } else {
              return bar.getMenuItems?.() || [];
            }
          },
        };
      }),
    };
  }

  function getStandardMenuItems(
    workItem: WorkItem,
    extraItems?: MenuItem[]
  ): MenuItem[] {
    return [
      getTitleMenuItem(workItem),
      ...(extraItems ? extraItems : []),
      getOpenMenuItem(workItem),
      getEditMenuItem(workItem),
    ];
  }

  function getProjectedEnd(item: WorkItem) {
    if (item.projectItem.iteration.loadState === "loaded") {
      const iterationId = item.projectItem.iteration.value;
      const iteration = context.data.fields.iteration.options.find(
        (i) => i.id === iterationId
      );
      if (iteration) {
        return dayjs(iteration.data.startDate)
          .add(Number(iteration.data.duration), "days")
          .format("YYYY-MM-DD");
      }
    }

    return undefined;
  }

  function collapseRows(rows: Row<Payload>[]): Row<Payload>[] {
    if (rows.length === 0) return rows;

    let minDate = Number.MAX_VALUE;
    let maxDate = Number.MIN_VALUE;

    for (const row of rows) {
      for (const bar of row.bars) {
        minDate = Math.min(minDate, dayjs(bar.start).unix());
        maxDate = Math.max(maxDate, dayjs(bar.end).unix());
      }
    }

    return [
      {
        bars: [
          {
            state: "completed",
            start: dayjs.unix(minDate).format("YYYY-MM-DD"),
            end: dayjs.unix(maxDate).format("YYYY-MM-DD"),
          },
        ],
      },
    ];
  }

  function cleanUpTitle(title: string) {
    return title
      .replace("[HLSL]", "")
      .replace("[Scenario]", "")
      .replace("[Deliverable]", "")
      .trim();
  }

  function getBurndownLabel(parent: WorkItem) {
    function getIssues(i: WorkItem): WorkItem[] {
      if (i.data.type !== "issue") return [];

      let issues: WorkItem[] = [];

      if (i !== parent) {
        // We don't burn down deliverables
        if (
          i.projectItem.kind.loadState !== "loaded" ||
          i.projectItem.kind.value === deliverableKindId
        )
          return [];

        issues = [i];
      }

      for (const subIssueId of i.data.subIssues) {
        const subIssue = context.data!.workItems[subIssueId];
        if (subIssue) issues = [...issues, ...getIssues(subIssue)];
      }

      return issues;
    }

    const issues = getIssues(parent);

    let active = 0;
    let open = 0;
    let closed = 0;

    const activeId = getStatusId("Active");

    for (const issue of issues) {
      const status = issue.projectItem.status;
      if (status === closedStatusId) closed++;
      else if (status == activeId) active++;
      else open++;
    }

    const total = active + open + closed;

    return `${closed}/${total} - ${active} active / ${open} open / ${closed} closed`;
  }

  const iterations: Iteration[] = $derived.by(() => {
    // Find the left-most bar
    const startDate = epics
      .map((e) =>
        e.scenarios.map((s) => s.rows.map((r) => r.bars.map((b) => b.start)))
      )
      .flat(3)
      .reduce((acc, value) => {
        const v = dayjs(value);
        return acc < v ? acc : v;
      }, dayjs());

    const iterations: Iteration[] = [];

    context.getIterationField("iteration").options.filter((o) => {
      const end = dayjs(o.data.startDate).add(
        Number(o.data.duration) - 1,
        "days"
      );
      if (end > startDate) {
        iterations.push({
          name: o.value,
          start: o.data.startDate,
          end: end.format("YYYY-MM-DD"),
        });
      }
    });

    return iterations;
  });

  const data: ExecutionTrackerData<Payload> = $derived.by(() => {
    return {
      iterations,
      epics,
    };
  });

  let editorWorkItem: WorkItem | undefined = $state(undefined);
  let editorOpen = $state(false);
</script>

<ExecutionTracker {data} />

<div class="mt-auto flex flex-row gap-2 p-2 max-h-fit">
  {#each Object.values(context.data.fields.epic.options) as epic}
    <button
      class="btn btn-sm {hiddenEpics.has(epic.value)
        ? 'preset-tonal'
        : 'preset-tonal-primary'}"
      onclick={() => {
        if (hiddenEpics.has(epic.value)) hiddenEpics.delete(epic.value);
        else hiddenEpics.add(epic.value);
      }}>{epic.value}</button
    >
  {/each}
</div>

<Portal>
  <WorkItemExtraDataEditor
    getInitialContent={() =>
      JSON.stringify(
        context.getWorkItemExtraData(editorWorkItem!.id),
        undefined,
        4
      )}
    onSave={(text) => {
      context.setWorkItemExtraData(editorWorkItem!.id, JSON.parse(text));
      editorOpen = false;
    }}
    onCancel={() => {
      editorOpen = false;
    }}
    open={editorOpen}
  >
    <h1>{$state.snapshot(editorWorkItem!.title)}</h1>
  </WorkItemExtraDataEditor>
</Portal>

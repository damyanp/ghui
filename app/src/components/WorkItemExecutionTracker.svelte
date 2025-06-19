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
    type Row,
    type Scenario,
  } from "./ExecutionTracker.svelte";
  import WorkItemExtraDataEditor from "./WorkItemExtraDataEditor.svelte";
  import { type WorkItemId } from "$lib/bindings/WorkItemId";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { MenuItem } from "./TreeTableContextMenu.svelte";
  import { tick } from "svelte";

  let context = getWorkItemContext();

  const startDate = "2025-02-09";

  type Payload = WorkItem;

  const epics: Epic<Payload>[] = $derived.by(() => {
    return Object.values(context.data.fields.epic.options).map(
      (epicFieldOption) => {
        return {
          name: epicFieldOption.value,
          targetDate: "TBD",
          scenarios: getScenarios(epicFieldOption.id),
        };
      }
    );
  });

  const scenarioKindId = $derived(
    context.data.fields.kind.options.find((o) => o.value === "Scenario")?.id
  );

  const deliverableKindId = $derived(
    context.data.fields.kind.options.find((o) => o.value === "Deliverable")?.id
  );

  const closedStatusId = $derived(
    context.data.fields.status.options.find((o) => o.value === "Closed")?.id
  );

  const statusOrdering = ["Active", "Ready", "Planning", undefined, "Closed"];
  const statusOrder = $derived.by(() => {
    return new Map(
      statusOrdering.map((name, index) => {
        return [
          context.data.fields.status.options.find((o) => o.value === name)?.id,
          index,
        ];
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

        let rows = getRows(scenario);

        if (isClosed) rows = collapseRows(rows);

        return {
          name: cleanUpTitle(scenario.title),
          rows,
          extraClasses: isClosed ? ["text-gray-500"] : undefined,
          getMenuItems: () => [getOpenMenuItem(scenario)],
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

    if (scenarios.length === 0) return [{ name: "TBD", rows: [] }];
    else return scenarios;
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

    if (rows.length === 0) return [{ bars: [] }];
    else return rows;

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
          const bars = <Bar<Payload>[]>extraData.bars;
          return {
            bars: bars.map((bar) => {
              return {
                ...bar,
                data: deliverable,
              };
            }),
          };
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
      const status = deliverable.projectItem.status;

      const state: BarState =
        status === closedStatusId
          ? "completed"
          : end === undefined
            ? "noDates"
            : dayjs(end) < dayjs()
              ? "offTrack"
              : "onTrack";

      let start: string | undefined = extraData.start;
      let estimate: number = extraData.estimate;

      if (!start) {
        start = (end && estimate)
          ? dayjs(end).subtract(estimate, "days").format("YYYY-MM-DD")
          : dayjs().add(1, "week").format("YYYY-MM-DD");
      }

      if (!end && estimate) {
        end = dayjs(start).add(estimate, "days").format("YYYY-MM-DD")
      }

      return {
        bars: [
          {
            state,
            label: cleanUpTitle(deliverable.title),
            start,
            end: end || defaultEnd,
            data: deliverable,
          },
        ],
      };
    }
  }

  function addStandardMenuItems(row: Row<Payload>): Row<Payload> {
    return {
      ...row,
      bars: row.bars.map((bar) => {
        return {
          ...bar,
          getMenuItems: () => {
            const items = bar.getMenuItems ? bar.getMenuItems() : [];

            if (bar.data) {
              items.push(getOpenMenuItem(bar.data), getEditMenuItem(bar.data));
            }

            return items;
          },
        };
      }),
    };
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

  const data: ExecutionTrackerData<Payload> = $derived.by(() => {
    return {
      startDate,
      epics: epics,
    };
  });

  let editorWorkItem: WorkItem | undefined = $state(undefined);
  let editorOpen = $state(false);
</script>

<ExecutionTracker {data} />

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

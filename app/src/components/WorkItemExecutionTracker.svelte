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

  type BarD = Bar & {
    deliverableId?: string;
    deliverableTitle?: string;
  };

  const epics: Epic<BarD>[] = $derived.by(() => {
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

  const defaultStart = startDate;
  const defaultEnd = dayjs().add(3, "month").format("YYYY-MM-DD");

  function getScenarios(epicId: FieldOptionId): Scenario<BarD>[] {
    const scenarios: Scenario<BarD>[] = Object.values(context.data.workItems)
      .filter((workItem): workItem is WorkItem => {
        if (!workItem) return false;

        return (
          workItem.projectItem.epic === epicId &&
          workItem.projectItem.kind.loadState === "loaded" &&
          workItem.projectItem.kind.value === scenarioKindId
        );
      })
      .map((scenario) => {
        return {
          name: cleanUpTitle(scenario.title),
          rows: getRows(scenario),
          id: scenario.id,
          getMenuItems: () => [getOpenMenuItem(scenario)],
        };
      })
      .sort((a, b) => getScenarioStartDate(a) - getScenarioStartDate(b));

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

  function getEditMenuItem(workItemId: WorkItemId): MenuItem {
    return {
      type: "action",
      title: "Edit...",
      action: () => {
        editorWorkItemId = workItemId;
        editorOpen = true;
      },
    };
  }

  function getScenarioStartDate(scenario: Scenario<BarD>) {
    if (scenario.rows.length === 0) return dayjs().unix();

    return scenario.rows
      .map((row) =>
        row.bars
          .map((bar) => dayjs(bar.start).unix())
          .reduce((a, b) => Math.min(a, b), Number.MAX_VALUE)
      )
      .reduce((a, b) => Math.min(a, b), Number.MAX_VALUE);
  }

  function getRows(scenario: WorkItem): Row<BarD>[] {
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

    function buildRowFromDeliverable(deliverable: WorkItem): Row<BarD> {
      const extraData = context.getWorkItemExtraData(deliverable.id);

      // If there are bars explicitly provided these override everything
      if (extraData) {
        if (extraData.bars) {
          const bars = <BarD[]>extraData.bars;
          return {
            bars: bars.map((bar) => {
              return {
                ...bar,
                deliverableId: deliverable.id,
                deliverableTitle: cleanUpTitle(deliverable.title),
              };
            }),
          };
        }
        if (extraData.split) {
          let bars: Partial<BarD>[] = [];
          let previousBar: Partial<BarD> | undefined = undefined;

          for (const entry of <Partial<BarD>[]>extraData.split) {
            let newBar = $state.snapshot(entry);
            if (!newBar.start) {
              newBar.start = previousBar?.end;
              if (!newBar.start) newBar.start = defaultStart;
            }

            if (previousBar && !previousBar.end) previousBar.end = newBar.start;

            bars.push(<BarD>{
              ...newBar,
              deliverableId: deliverable.id,
              deliverableTitle: cleanUpTitle(deliverable.title),
            });
            previousBar = bars[bars.length - 1];
          }

          if (bars.length > 0 && !bars[bars.length - 1].end) {
            bars[bars.length - 1].end = getProjectedEnd(deliverable);
          }
          console.log(bars);
          return { bars: <BarD[]>bars };
        }
      }

      const projectedEnd = getProjectedEnd(deliverable);
      const status = deliverable.projectItem.status;

      const state: BarState =
        status === closedStatusId
          ? "completed"
          : projectedEnd === undefined
            ? "noDates"
            : dayjs(projectedEnd) < dayjs()
              ? "offTrack"
              : "onTrack";

      let start: string | undefined = extraData.start;

      if (!start) {
        start = projectedEnd
          ? dayjs(projectedEnd).subtract(1, "week").format("YYYY-MM-DD")
          : dayjs().format("YYYY-MM-DD");
      }

      return {
        bars: [
          {
            state,
            label: cleanUpTitle(deliverable.title),
            start,
            end: projectedEnd || defaultEnd,
            deliverableId: deliverable.id,
            deliverableTitle: cleanUpTitle(deliverable.title),
          },
        ],
      };
    }
  }

  function addStandardMenuItems(row: Row<BarD>): Row<BarD> {
    return {
      ...row,
      bars: row.bars.map((bar) => {
        return {
          ...bar,
          getMenuItems: () => {
            const items = bar.getMenuItems ? bar.getMenuItems() : [];

            if (bar.deliverableId) {
              items.push(
                getOpenMenuItem(context.data.workItems[bar.deliverableId]!),
                getEditMenuItem(bar.deliverableId)
              );
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

  function cleanUpTitle(title: string) {
    return title
      .replace("[HLSL]", "")
      .replace("[Scenario]", "")
      .replace("[Deliverable]", "")
      .trim();
  }

  const data: ExecutionTrackerData<BarD> = $derived.by(() => {
    return {
      startDate,
      epics: epics,
    };
  });

  let editorWorkItemId: string | undefined = $state(undefined);
  let editorOpen = $state(false);
</script>

<ExecutionTracker {data} />

<WorkItemExtraDataEditor
  getInitialContent={() =>
    JSON.stringify(
      context.getWorkItemExtraData(editorWorkItemId!),
      undefined,
      4
    )}
  onSave={(text) => {
    context.setWorkItemExtraData(editorWorkItemId!, JSON.parse(text));
    editorOpen = false;
  }}
  onCancel={() => {
    editorOpen = false;
  }}
  open={editorOpen}
>
  <h1>{context.data!.workItems[$state.snapshot(editorWorkItemId)!]?.title}</h1>
</WorkItemExtraDataEditor>

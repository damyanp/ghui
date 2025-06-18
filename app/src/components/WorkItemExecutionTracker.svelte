<script lang="ts">
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { WorkItem } from "$lib/bindings/WorkItem";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
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

  let context = getWorkItemContext();

  const startDate = "2025-02-09";

  const epics: Epic[] = $derived.by(() => {
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

  function getScenarios(epicId: FieldOptionId): Scenario[] {
    const scenarios = Object.values(context.data.workItems)
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
        };
      })
      .sort((a, b) => getScenarioStartDate(a) - getScenarioStartDate(b));

    if (scenarios.length === 0) return [{ name: "TBD", rows: [] }];
    else return scenarios;
  }

  function getScenarioStartDate(scenario: Scenario) {
    if (scenario.rows.length === 0) return dayjs().unix();

    return scenario.rows
      .map((row) =>
        row.bars
          .map((bar) => dayjs(bar.start).unix())
          .reduce((a, b) => Math.min(a, b), Number.MAX_VALUE)
      )
      .reduce((a, b) => Math.min(a, b), Number.MAX_VALUE);
  }

  function getRows(scenario: WorkItem): Row[] {
    if (scenario.data.type !== "issue") return [];

    const rows = scenario.data.subIssues
      .map((id) => {
        return context.data.workItems[id];
      })
      .filter((i): i is WorkItem => {
        if (!i) return false;

        return (
          i.projectItem.kind.loadState === "loaded" &&
          i.projectItem.kind.value === deliverableKindId
        );
      })
      .map((deliverable) => {
        const extraData = context.getWorkItemExtraData(deliverable.id);

        // If there are bars explicitly provided these override everything
        if (extraData) {
          if (extraData.bars) {
            const bars = <Bar[]>extraData.bars;
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
            let bars = [];
            let previousBar: Partial<Bar> | undefined = undefined;

            for (const entry of <Partial<Bar>[]>extraData.split) {
              let newBar = $state.snapshot(entry);
              if (!newBar.start) {
                newBar.start = previousBar?.end;
                if (!newBar.start) newBar.start = defaultStart;
              }

              if (previousBar && !previousBar.end)
                previousBar.end = newBar.start;

              bars.push({
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
            return { bars: <Bar[]>bars };
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

        return {
          bars: [
            {
              state,
              label: cleanUpTitle(deliverable.title),
              start: projectedEnd ? dayjs(projectedEnd).subtract(1, "week").format("YYYY-MM-DD") : dayjs().format("YYYY-MM-DD"),
              end: projectedEnd || defaultEnd,
              deliverableId: deliverable.id,
              deliverableTitle: cleanUpTitle(deliverable.title),
            },
          ],
        };
      });

    if (rows.length === 0) return [{ bars: [] }];
    else return rows;
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

  const data: ExecutionTrackerData = $derived.by(() => {
    return {
      startDate,
      epics: epics,
    };
  });
</script>

<ExecutionTracker {data}>
  <!-- {#snippet scenarioEditor(scenario: Scenario & { id?: string })}
    {#if scenario.id}
      {@render editor(scenario.id, scenario.name)}
    {/if}
  {/snippet} -->

  {#snippet barEditor(
    bar: Bar & { deliverableId?: string; deliverableTitle?: string }
  )}
    {#if bar.deliverableId && bar.deliverableTitle}
      {@render editor(bar.deliverableId, bar.deliverableTitle)}
    {/if}
  {/snippet}
</ExecutionTracker>

{#snippet editor(id: WorkItemId, label: string)}
  <div class="inline group-hover:opacity-100 transition-opacity opacity-0">
    <WorkItemExtraDataEditor
      content={JSON.stringify(context.getWorkItemExtraData(id), undefined, 4)}
      onSave={(text) => {
        context.setWorkItemExtraData(id, JSON.parse(text));
      }}
    >
      <h1>{label}</h1>
    </WorkItemExtraDataEditor>
  </div>
{/snippet}

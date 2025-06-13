<script lang="ts" module>
  import { getContext, setContext } from "svelte";
  import dayjs from "dayjs";

  export type Data = {
    epics: Epic[];
    startDate: Date;
  };

  export type Date = string;

  export type Epic = {
    name: string;
    targetDate: Date;
    scenarios: Scenario[];
  };

  export type Scenario = {
    name: string;
    rows: Row[];
  };

  export type Row = {
    bars: Bar[];
  };

  export type Bar = {
    state: BarState;
    label?: string;
    start: Date;
    end: Date;
  };

  export type BarState =
    | "completed"
    | "onTrack"
    | "atRisk"
    | "offTrack"
    | "notStarted"
    | "noDates";

  const key = Symbol("ExecutionTrackerContext");

  export function setExecutionTrackerContext(c: ExecutionTrackerContext) {
    setContext(key, c);
    return c;
  }

  function getExecutionTrackerContext() {
    return getContext(key) as ExecutionTrackerContext;
  }

  export class ExecutionTrackerContext {
    scale = $state(0.0001);
  }
</script>

<script lang="ts">
  import { ZoomIn, ZoomInIcon, ZoomOut, ZoomOutIcon } from "@lucide/svelte";

  let { data }: { data: Data } = $props();

  let context = getExecutionTrackerContext();

  const [minDate, maxDate] = $derived.by(() => {
    let minDate = Number.MAX_VALUE;
    let maxDate = Number.MIN_VALUE;

    for (const epic of data.epics) {
      for (const scenario of epic.scenarios) {
        for (const row of scenario.rows) {
          for (const bar of row.bars) {
            if (bar.start) {
              minDate = Math.min(minDate, dayjs(bar.start).unix());
              maxDate = Math.max(maxDate, dayjs(bar.start).unix());
            }
            if (bar.end) {
              minDate = Math.min(minDate, dayjs(bar.end).unix());
              maxDate = Math.max(maxDate, dayjs(bar.end).unix());
            }
          }
        }
      }
    }

    return [minDate, maxDate];
  });

  const [minX, maxX] = $derived([minDate * context.scale, maxDate * context.scale]);

  function convertDate(date: string): number {
    return dayjs(date).unix() * context.scale;
  }

  function getBarFillStyle(state: BarState): string {
    switch (state) {
      case "atRisk":
        return "background-color: #f7c7ac;";
      case "completed":
        return "background-color: #c0e6f5;";
      case "noDates":
        return "background-color: #d9d9d9; background-size: 4px 4px; background-image: linear-gradient(45deg, #9d9d9d 2px, transparent 0);";
      case "notStarted":
        return "background-color: #d9d9d9;";
      case "offTrack":
        return "background-color: #ff7c80;";
      case "onTrack":
        return "background-color: #c1f0c8;";
    }
  }

  function getEpicRowSpan(epic: Epic) {
    return Math.max(
      1,
      epic.scenarios.reduce((prev, current) => {
        return prev + Math.max(1, current.rows.length);
      }, 0)
    );
  }

  const totalRows = $derived(
    data.epics.reduce((prev, current) => {
      return prev + getEpicRowSpan(current);
    }, 0)
  );

  const chartWidth = $derived(maxX - minX);

  const dates = $derived.by(() => {
    let dates = [];

    let date = dayjs(data.startDate);
    let endDate = dayjs.unix(maxDate).add(1, "day");

    while (date < endDate) {
      dates.push({
        value: date.unix() * context.scale - minX,
        label: date.format("MM-DD"),
      });
      date = date.add(7, "day");
    }

    return dates;
  });

  function getEpicFillStyle(index: number) {
    if (index % 2 == 0) {
      return "background-color: #202020;";
    } else {
      return "background-color: #203030;";
    }
  }
</script>

<div
  class="grid gap-1 overflow-y-auto"
  style={`grid-template-rows: repeat(${totalRows + 2}, 2.5em); grid-template-columns: repeat(3, max-content) 1fr`}
>
  <div class="col-start-4 row-start-1 z-[100] group">
    <div class="flex w-fit h-[2.5em] fixed right-[2em] items-center gap-1">
      <button
        class="btn-icon preset-filled opacity-0 transition-opacity group-hover:opacity-100"
        onclick={() => (context.scale = context.scale * 1.1)}><ZoomIn /></button
      >
      <button
        class="btn-icon preset-filled opacity-0 transition-opacity group-hover:opacity-100"
        onclick={() => (context.scale = context.scale / 1.1)}><ZoomOut /></button
      >
    </div>
  </div>

  <!-- The first three, frozen, columns -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-1 col-end-4 grid left-0 sticky bg-surface-50-950 z-40 border-r"
    style={`grid-row: 1 / span ${totalRows + 2};`}
  >
    <div class="font-bold p-1 bg-teal-800">Product Epic</div>
    <div class="font-bold p-1 bg-teal-800">Target Date</div>
    <div class="font-bold p-1 bg-teal-800">Engineering Scenarios</div>

    {#each data.epics as epic, epicIndex}
      <div
        class="col-start-1 p-1"
        style={`grid-row: span ${getEpicRowSpan(epic)}; ${getEpicFillStyle(epicIndex)}`}
      >
        {epic.name}
      </div>
      <div
        class="p-1"
        style={`grid-row: span ${getEpicRowSpan(epic)}; ${getEpicFillStyle(epicIndex)}`}
      >
        {epic.targetDate}
      </div>
      {#each epic.scenarios as scenario}
        <div
          class="p-1 col-start-3"
          style={`grid-row: span ${scenario.rows.length}; ${getEpicFillStyle(epicIndex)}`}
        >
          {scenario.name}
        </div>
      {/each}
    {/each}
  </div>

  <!-- The date strip in row 1, the vertical lines for dates, and the line for today -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid"
    style={`grid-row: 1 / span ${totalRows + 2};`}
  >
    <!-- Vertical lines -->
    <div
      class="col-start-1 relative"
      style={`grid-row: 1 / span ${totalRows + 1};`}
    >
      {#each dates as date}
        <div
          class="absolute border-l border-surface-400-600 h-full z-10 py-5"
          style={`left: ${date.value}px;`}
        >
          &nbsp;
        </div>
      {/each}

      <div
        class="absolute border-l-4 border-teal-300 h-full py-5 border-dashed"
        style={`left: ${dayjs().unix() * context.scale - minX}px;`}
      >
        &nbsp;
      </div>
    </div>

    <!-- Row 1's date labels -->
    <div class="row-start-1 col-start-1 relative text-white bg-teal-800 z-0">
      {#each dates as date}
        <div
          class="absolute"
          style={`left: ${date.value}px; transform: translate(-50%,0)`}
        >
          {date.label}
        </div>
      {/each}
    </div>
  </div>

  <!-- Background fill for each scenario - color comes from the epic, height
       determined by number of rows in scenario -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid"
    style={`grid-row: 2 / span ${totalRows + 2};`}
  >
    {#each data.epics as epic, epicIndex}
      {#each epic.scenarios as scenario}
        <div
          style={`${getEpicFillStyle(epicIndex)} grid-row: span ${scenario.rows.length};`}
        >
          &nbsp;
        </div>
      {/each}
    {/each}
  </div>

  <!-- The bars themselves -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid"
    style={`grid-row: 2 / span ${totalRows + 2};`}
  >
    {#each data.epics as epic, epicIndex}
      {#each epic.scenarios as scenario}
        {#each scenario.rows as row}
          <div
            class="col-start-1 text-xs relative text-black"
            style={`width: ${maxX - minX}px`}
          >
            {#each row.bars as bar}
              {@const start = convertDate(bar.start) - minX}
              {@const width = convertDate(bar.end) - convertDate(bar.start)}
              <div
                class="absolute content-center text-center h-full z-10"
                style={`left: ${start}px; max-width: ${width}px; width: ${width}px;`}
              >
                <div
                  class="w-full h-[2em] text-nowrap overflow-ellipsis overflow-clip content-center text-center rounded-r-xl"
                  style={`${getBarFillStyle(bar.state)};`}
                >
                  {#if bar.label}
                    {bar.label}
                  {:else}
                    &nbsp;
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/each}
      {/each}
    {/each}
  </div>
</div>

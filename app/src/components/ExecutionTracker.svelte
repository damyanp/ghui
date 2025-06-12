<script lang="ts" module>
  export type Data = {
    epics: Epic[];
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
</script>

<script lang="ts">
  let { data }: { data: Data } = $props();

  let scale = $state(0.0000001);

  const [minDate, maxDate] = $derived.by(() => {
    let minDate = Number.MAX_VALUE;
    let maxDate = Number.MIN_VALUE;

    for (const epic of data.epics) {
      for (const scenario of epic.scenarios) {
        for (const row of scenario.rows) {
          for (const bar of row.bars) {
            if (bar.start) {
              minDate = Math.min(minDate, convertDate(bar.start));
              maxDate = Math.max(maxDate, convertDate(bar.start));
            }
            if (bar.end) {
              minDate = Math.min(minDate, convertDate(bar.end));
              maxDate = Math.max(maxDate, convertDate(bar.end));
            }
          }
        }
      }
    }

    return [minDate, maxDate];
  });

  function convertDate(date: string): number {
    const d = Date.parse(date);
    return d.valueOf() * scale;
  }

  function getFillStyle(state: BarState): string {
    switch (state) {
      case "atRisk":
        return "background-color: #f7c7ac;";
      case "completed":
        return "background-color: #c0e6f5;";
      case "noDates":
        return "background-color: #d9d9d9; background-size: 8px 8px; background-image: radial-gradient(black 1px, transparent 0);";
      case "notStarted":
        return "background-color: #d9d9d9;";
      case "offTrack":
        return "background-color: #ff7c80;";
      case "onTrack":
        return "background-color: #c1f0c8;";
    }
  }

  function getEpicRowSpan(epic: Epic) {
    return epic.scenarios.reduce((prev, current) => {
      return prev + current.rows.length;
    }, 0);
  }

  const totalRows = $derived(
    data.epics.reduce((prev, current) => {
      return prev + getEpicRowSpan(current);
    }, 0)
  );

  const chartWidth = $derived(maxDate - minDate);
</script>

<div>
  <div
    class="grid gap-1 overflow-y-auto"
    style={`grid-template-rows: repeat(${totalRows + 2}, 2em); grid-template-columns: repeat(3, max-content) 1fr`}
  >
    <div
      class="grid-cols-subgrid grid-rows-subgrid col-start-1 col-end-4 grid left-0 sticky bg-surface-50-950 z-50 border-r"
      style={`grid-row: 1 / span ${totalRows + 2};`}
    >
      <div class="font-bold p-1">Product Epic</div>
      <div class="font-bold p-1">Target Date</div>
      <div class="font-bold p-1">Engineering Scenarios</div>

      {#each data.epics as epic}
        <div
          class="col-start-1 p-1"
          style={`grid-row: span ${getEpicRowSpan(epic)}`}
        >
          {epic.name}
        </div>
        <div class="p-1" style={`grid-row: span ${getEpicRowSpan(epic)}`}>
          {epic.targetDate}
        </div>
        {#each epic.scenarios as scenario}
          <div
            class="p-1 col-start-3"
            style={`grid-row: span ${scenario.rows.length}`}
          >
            {scenario.name}
          </div>
        {/each}
      {/each}
    </div>

    <div
      class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid overflow-x-auto overflow-y-clip"
      style={`grid-row: 1 / span ${totalRows + 2};`}
    >
      <div class="row-start-1 col-start-1 relative">
        <div
          class="absolute left-[100px] w-[100px] h-full bg-surface-500 text-center my-auto text-black text-xs"
        >
          Dates go here
        </div>
      </div>

      {#each data.epics as epic}
        {#each epic.scenarios as scenario}
          {#each scenario.rows as row}
            <div class="col-start-1 p-1 text-xs relative text-black">
              {#each row.bars as bar}
                {@const start = convertDate(bar.start) - minDate}
                {@const width = convertDate(bar.end) - convertDate(bar.start)}
                <div
                  class="absolute h-full overflow-ellipsis overflow-clip ztext-nowrap content-center text-center rounded-r-full"
                  style={`left: ${start}px; max-width: ${width}px; width: ${width}px; ${getFillStyle(bar.state)};`}
                >
                  {#if bar.label}
                    {bar.label}
                  {:else}
                    &nbsp;
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
        {/each}
      {/each}
    </div>
  </div>
</div>

<style lang="postcss">
  @reference "../app.css";

  div {
    text-wrap: nowrap;
  }
</style>

<script lang="ts" module>
  export type BarState =
    | "completed"
    | "onTrack"
    | "atRisk"
    | "offTrack"
    | "notStarted"
    | "noDates";

  export type Date = string;

  export type Bar = {
    state: BarState;
    label?: string;
    start: Date;
    end: Date;
  };

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

  export type Data = {
    epics: Epic[];
  };
</script>

<script lang="ts">
  import { DateField } from "bits-ui";

  let { data }: { data: Data } = $props();

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
    return d.valueOf() / 100000000;
  }

  function getFill(state: BarState): string {
    switch (state) {
      case "atRisk":
        return "#f7c7ac";
      case "completed":
        return "#c0e6f5";
      case "noDates":
        return "#d9d9d9";
      case "notStarted":
        return "#d9d9d9";
      case "offTrack":
        return "#ff7c80 ";
      case "onTrack":
        return "#c1f0c8";
    }
  }

  function getEpicRowSpan(epic: Epic) {
    return epic.scenarios.reduce((prev, current) => {
      return prev + current.rows.length;
    }, 0);
  }
</script>

<div class="overflow-x-auto">
  <div class="grid gap-1" style="grid-template-columns: auto auto auto auto">
    <div class="font-bold p-1">Product Epic</div>
    <div class="font-bold p-1">Target Date</div>
    <div class="font-bold p-1">Engineering Scenarios</div>
    <div class="w-[2048px]"></div>

    {#each data.epics as epic}
      <div class="col-start-1 p-1" style={`grid-row: span ${getEpicRowSpan(epic)}`}>
        {epic.name}
      </div>
      <div class="p-1" style={`grid-row: span ${getEpicRowSpan(epic)}`}>
        {epic.targetDate}
      </div>
      {#each epic.scenarios as scenario}
        <div class="p-1" style={`grid-row: span ${scenario.rows.length}`}>
          {scenario.name}
        </div>
        {#each scenario.rows as row}
          <div class="col-start-4 w-[2048px] p-1 text-xs">
            {#each row.bars as bar}
              {@const start = convertDate(bar.start)}
              {@const end = convertDate(bar.end)}
              {#if bar.label}
                <span>{bar.label}</span>
              {/if}
            {/each}
          </div>
        {/each}
      {/each}
    {/each}
  </div>
</div>

<style lang="postcss">
  @reference "../app.css";

  div {
    text-wrap-mode: nowrap;
  }
</style>

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
    start?: Date;
    end?: Date;
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
  let { data }: { data: Data } = $props();
</script>

<div class="overflow-x-auto">
  <div class="grid gap-1" style="grid-template-columns: auto auto auto 1fr ">
    
    <div>Product Epic</div>
    <div>Target Date</div>
    <div>Engineering Scenarios</div>
    <div>&nbsp;</div>

    {#each data.epics as epic}
      <div>{epic.name}</div>
      <div>{epic.targetDate}</div>
      <div class="col-start-3 col-end-5 grid grid-cols-subgrid gap-1">
        {#each epic.scenarios as scenario}
          <div class="">{scenario.name}</div>
          <div class="col-start-5 col-end-6 grid-cols-subgrid w-full">
            {#each scenario.rows as row}
            <div>
                {#each row.bars as bar}
                <div class="p-x-2 bg-amber-950 w-fit mx-2 inline">{bar.label}&nbsp;</div>
                {/each}
            </div>
            {/each}
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style lang="postcss">
    @reference "../app.css";
</style>

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
    deliverables: Deliverable[];
  };

  export type Deliverable = {
    name: string;
    bars: Bar[];
  };

  export type Data = {
    epics: Epic[]
  };
</script>

<script lang="ts">
  let { data }: { data: Data } = $props();
</script>


{#each data.epics as epic}
{epic.name} / {epic.targetDate}

<svg class="border">
    <circle cx="50" cy="50" r="40" />
</svg>
{/each}


<script lang="ts">
  /*
    Use "npm run dev" and then browse to localhost/test_xt to test this without
    needing a full tauri stack running.
    */

  import ExecutionTracker, {
    type Bar,
    type Data,
  } from "../../components/ExecutionTracker.svelte";

  const data: Data<never> = {
    iterations: [
      {
        name: "Y25-S01",
        start: "2025-01-13",
        end: "2025-01-26",
      },
      {
        name: "Y25-S01",
        start: "2025-01-13",
        end: "2025-01-26",
      },
      {
        name: "Y25-S02",
        start: "2025-01-27",
        end: "2025-02-09",
      },
      {
        name: "Y25-S03",
        start: "2025-02-10",
        end: "2025-02-23",
      },
      {
        name: "Y25-S04",
        start: "2025-02-24",
        end: "2025-03-09",
      },
      {
        name: "Y25-S05",
        start: "2025-03-10",
        end: "2025-03-23",
      },
      {
        name: "Y25-S06",
        start: "2025-03-24",
        end: "2025-04-06",
      },
      {
        name: "Y25-S07",
        start: "2025-04-07",
        end: "2025-04-20",
      },
      {
        name: "Y25-S08",
        start: "2025-04-21",
        end: "2025-05-04",
      },
      {
        name: "Y25-S09",
        start: "2025-05-05",
        end: "2025-05-18",
      },
      {
        name: "Y25-S10",
        start: "2025-05-19",
        end: "2025-06-01",
      },
      {
        name: "Y25-S11",
        start: "2025-06-02",
        end: "2025-06-15",
      },
      {
        name: "Y25-S12",
        start: "2025-06-16",
        end: "2025-06-29",
      },
      {
        name: "Y25-S13",
        start: "2025-06-30",
        end: "2025-07-13",
      },
      {
        name: "Y25-S14",
        start: "2025-07-14",
        end: "2025-07-27",
      },
      {
        name: "Y25-S15",
        start: "2025-07-28",
        end: "2025-08-10",
      },
      {
        name: "Y25-S16",
        start: "2025-08-11",
        end: "2025-08-24",
      },
      {
        name: "Y25-S17",
        start: "2025-08-25",
        end: "2025-09-07",
      },
    ],
    epics: [
      {
        name: "Particle Life",
        targetDate: "2025-04-31",
        scenarios: [
          {
            name: "Constant Buffers",
            rows: [
              {
                bars: [
                  {
                    state: "completed",
                    label: "Implementation",
                    start: "2025-01-12",
                    end: "2025-04-06",
                  },
                  {
                    state: "noDates",
                    start: "2025-04-06",
                    end: "2025-04-20",
                  },
                ],
              },
            ],
          },

          {
            name: "Particle Life Finalization",
            rows: [
              {
                bars: [
                  {
                    state: "completed",
                    start: "2025-04-20",
                    end: "2025-05-04",
                    label: "0 of 2 Bugs Resolved",
                  },
                ],
              },
            ],
          },
        ],
      },
      {
        name: "DML Demo",
        targetDate: "2025-12-31",
        scenarios: [
          {
            name: "Testing Infra",
            rows: [
              {
                bars: [
                  {
                    state: "completed",
                    start: "2025-03-02",
                    end: "2025-03-16",
                    label: "Implementation",
                  },
                ],
              },
            ],
          },
          {
            name: "Language: Casting/Overload",
            rows: [
              {
                bars: [
                  {
                    state: "completed",
                    start: "2025-03-16",
                    end: "2025-04-06",
                    label: "Implementation",
                  },
                  { state: "noDates", start: "2025-04-06", end: "2025-04-27" },
                ],
              },
            ],
          },
          {
            name: "Implicit Resource Bindings",
            rows: [
              {
                bars: [
                  {
                    label: "Requirements/Test Cases",
                    state: "completed",
                    start: "2025-03-16",
                    end: "2025-03-30",
                  },
                  {
                    label: "Design and Planning",
                    state: "completed",
                    start: "2025-03-30",
                    end: "2025-04-13",
                  },
                  {
                    label: "Implementation by June 1st",
                    state: "completed",
                    start: "2025-04-13",
                    end: "2025-06-01",
                  },
                ],
              },
            ],
          },
          {
            name: "Root Signatures",
            rows: [
              {
                bars: [
                  {
                    label: "Requirements/Test Cases/Design",
                    state: "completed",
                    start: "2025-03-16",
                    end: "2025-04-06",
                  },
                ],
              },
              {
                bars: [
                  {
                    label: "Implementation by June 29th",
                    state: "onTrack",
                    start: "2025-03-30",
                    end: "2025-06-29",
                  },
                ],
              },
            ],
          },
          {
            name: "Shader Flags",
            rows: [
              {
                bars: [
                  {
                    label: "Requirements/Test Cases",
                    state: "completed",
                    start: "2025-03-16",
                    end: "2025-03-30",
                  },
                  {
                    label: "Planning",
                    state: "completed",
                    start: "2025-03-30",
                    end: "2025-04-06",
                  },
                  {
                    label: "Implementation",
                    state: "completed",
                    start: "2025-04-06",
                    end: "2025-04-20",
                  },
                  { state: "noDates", start: "2025-04-20", end: "2025-04-27" },
                ],
              },
            ],
          },
          {
            name: "ML Preview Exec Text",
            rows: [
              {
                bars: [
                  {
                    label: "Requirements/Planning",
                    state: "completed",
                    start: "2025-03-23",
                    end: "2025-03-30",
                  },
                  {
                    label: "Implementation",
                    state: "completed",
                    start: "2025-03-30",
                    end: "2025-04-20",
                  },
                  { state: "noDates", start: "2025-04-20", end: "2025-04-27" },
                ],
              },
            ],
          },
          {
            name: "Intrinsics Functional Tests",
            rows: [
              {
                bars: [
                  {
                    label: "Scenario Planning",
                    state: "completed",
                    start: "2025-04-06",
                    end: "2025-04-13",
                  },
                  {
                    label: "Update Testing Infrastructure",
                    state: "completed",
                    start: "2025-04-13",
                    end: "2025-04-27",
                  },
                  { state: "noDates", start: "2025-04-27", end: "2025-06-01" },
                ],
              },
              {
                bars: [
                  {
                    label: "Implementation Planning",
                    state: "completed",
                    start: "2025-04-27",
                    end: "2025-05-25",
                  },
                  {
                    label: "Implementation by Aug 31st",
                    state: "onTrack",
                    start: "2025-05-25",
                    end: "2025-08-31",
                  },
                ],
              },
            ],
          },
          {
            name: "Legalization and Lowering",
            rows: [
              {
                bars: [
                  {
                    label: "8 of 17 Bugs Resolved, 4 Active",
                    state: "onTrack",
                    start: "2025-04-27",
                    end: "2025-06-01",
                  },
                ],
              },
              {
                bars: [
                  {
                    label: "Planning Symbol Visibility",
                    state: "noDates",
                    start: "2025-05-18",
                    end: "2025-06-01",
                  },
                ],
              },
            ],
          },
          {
            name: "Buffer Resources",
            rows: [
              {
                bars: [
                  {
                    label: "Scenario Planning",
                    start: "2025-04-27",
                    end: "2025-06-01",
                    state: "noDates",
                  },
                ],
              },
            ],
          },
          {
            name: "ML Demo Finalization",
            rows: [
              {
                bars: [
                  {
                    label: "3 of 8 Bugs Resolved, 1 Active",
                    state: "onTrack",
                    start: "2025-04-27",
                    end: "2025-06-01",
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  };
</script>

<div class="py-10 px-1">
  <ExecutionTracker {data} />
</div>

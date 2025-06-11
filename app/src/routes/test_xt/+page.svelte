<script lang="ts">
  /*
    Use "npm run dev" and then browse to localhost/test_xt to test this without
    needing a full tauri stack running.
    */

  import ExecutionTracker, { type Data } from "../../components/ExecutionTracker.svelte";

  const data : Data = {
    epics: [
      {
        name: "DML Demo",
        targetDate: "2025-12-31",
        scenarios: [
          {
            name: "Intrinsics Functional Tests",
            deliverables: [
              {
                name: "Intrinsics Functional Tests Scenario Planning",
                bars: [
                  {
                    state: "completed",
                    start: "2025-04-07",
                    end: "2025-04-27",
                  },
                ],
              },
              {
                name: "Update Testing Infrastructure",
                bars: [
                  {
                    label: "planning",
                    state: "completed",
                    start: "2025-04-14",
                    end: "2025-04-29",
                  },
                  {
                    label: "active",
                    state: "noDates",
                  },
                ],
              },
              {
                name: "Implement Intrinsic tests in offload test suite",
                bars: [
                  {
                    label: "planning",
                    state: "completed",
                    start: "2025-04-29",
                    end: "2025-05-25",
                  },
                  { label: "active", state: "onTrack", end: "2025-08-31" },
                ],
              },
            ],
          },
        ],
      },
    ],
  };
</script>

<div class="flex justify-center items-center h-[100vh]">
  <div class="border min-w-[75vw] min-h-[75vh]">
    <ExecutionTracker data={data}/>
  </div>
</div>

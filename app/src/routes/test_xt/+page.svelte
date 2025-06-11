<script lang="ts">
  /*
    Use "npm run dev" and then browse to localhost/test_xt to test this without
    needing a full tauri stack running.
    */

  import ExecutionTracker from "../../components/ExecutionTracker.svelte";
</script>

<div class="flex justify-center items-center h-[100vh]">
  <div class="border min-w-[75vw] min-h-[75vh]">
    <ExecutionTracker />
  </div>
</div>


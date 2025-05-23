<script lang="ts">
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { Bubbles } from "@lucide/svelte";
  import { scale } from "svelte/transition";

  let context = getWorkItemContext();
</script>

<button class="btn p-1" title="Sanitize" onclick={() => context.sanitize()}>
  <Bubbles />
</button>

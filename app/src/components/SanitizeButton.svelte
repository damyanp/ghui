<script lang="ts">
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { Bubbles } from "@lucide/svelte";
  import { scale } from "svelte/transition";
  import AppBarButton from "./AppBarButton.svelte";

  let context = getWorkItemContext();
</script>

<AppBarButton icon={Bubbles} text="Sanitize" onclick={() => context.sanitize()} />

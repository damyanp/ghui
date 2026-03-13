<script lang="ts">
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { Bubbles } from "@lucide/svelte";
  import AppBarButton from "./AppBarButton.svelte";

  let { disabled = false }: { disabled?: boolean } = $props();

  let context = getWorkItemContext();
</script>

<AppBarButton icon={Bubbles} text="Sanitize" {disabled} onclick={() => context.sanitize()} />

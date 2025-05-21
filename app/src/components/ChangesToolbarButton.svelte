<script lang="ts">
  import type { Changes } from "$lib/bindings/Changes";

  let { changes }: { changes: Changes } = $props();
  
</script>

{#if Object.keys(changes.data).length > 0}
  {Object.keys(changes.data).length} changes
{/if}

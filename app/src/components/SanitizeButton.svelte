<script lang="ts">
  import { Bubbles } from "@lucide/svelte";
  import AppBarButton from "./AppBarButton.svelte";

  let {
    disabled = false,
    onclick,
  }: { disabled?: boolean; onclick: () => void } = $props();
</script>

<AppBarButton icon={Bubbles} text="Sanitize" {disabled} {onclick} />

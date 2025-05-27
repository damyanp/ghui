<script lang="ts">
  import { Switch } from "@skeletonlabs/skeleton-svelte";

  type Props = { label: string; checked: boolean };

  let { checked = $bindable(), ...props }: Props = $props();

  function onCheckedChange(details: { checked: boolean }) {
    checked = details.checked;
  }
</script>

<div class="btn flex flex-col p-0.5 mx-1">
  <div class="h-[24px]">
    <Switch {...props} {onCheckedChange} />
  </div>
  <span class="text-xs">{props.label}</span>
</div>

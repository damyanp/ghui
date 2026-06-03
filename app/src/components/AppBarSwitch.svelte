<script lang="ts">
  import { Switch } from "@skeletonlabs/skeleton-svelte";

  type Props = { label: string; checked: boolean };

  let { checked = $bindable(), label }: Props = $props();

  function onCheckedChange(details: { checked: boolean }) {
    checked = details.checked;
  }
</script>

<div class="btn flex flex-col p-0.5 mx-1">
  <div class="h-[24px]">
    <Switch {checked} {onCheckedChange}>
      <Switch.Control>
        <Switch.Thumb />
      </Switch.Control>
      <Switch.HiddenInput />
    </Switch>
  </div>
  <span class="text-xs">{label}</span>
</div>

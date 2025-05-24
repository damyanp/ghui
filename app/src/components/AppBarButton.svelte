<script lang="ts">
  import { type Icon } from "@lucide/svelte";
  import type { ClassValue, HTMLButtonAttributes } from "svelte/elements";

  type Props = {
    icon: typeof Icon;
    iconClass?: ClassValue;
    text: string;
  } & HTMLButtonAttributes;

  let { icon: MyIcon, text, iconClass, ...otherProps }: Props = $props();
</script>

<button class="btn rounded p-0.5 mx-1 flex-col" {...otherProps}>
  <MyIcon class={iconClass} />
  <span class="text-xs">{text}</span>
</button>

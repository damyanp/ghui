<script lang="ts">
  import { type Icon } from "@lucide/svelte";
  import type { ClassValue, HTMLButtonAttributes } from "svelte/elements";

  type Props = {
    icon: typeof Icon;
    iconClass?: ClassValue;
    text: string;
    badge?: number;
  } & HTMLButtonAttributes;

  let { icon: MyIcon, text, iconClass, badge, ...otherProps }: Props = $props();
</script>

<button class="btn rounded p-0.5 mx-1 flex-col relative" {...otherProps}>
  <MyIcon class={iconClass} />
  {#if badge !== undefined}
    <span
      class="absolute -bottom-1 -right-1 bg-primary-500 text-white text-[0.6rem] leading-none min-w-3.5 h-3.5 flex items-center justify-center rounded-full px-0.5"
      aria-label="{badge} pending change{badge !== 1 ? 's' : ''}"
      >{badge}</span
    >
  {/if}
  <span class="text-xs">{text}</span>
</button>

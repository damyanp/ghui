<script lang="ts">
  import { RefreshCw } from "@lucide/svelte";
  import type { NumericRange } from "@sveltejs/kit";

  let props: {
    onclick: (e:MouseEvent) => void;
    progress: number; // 0 = not refreshing, otherwise counts towards 0
  } = $props();

  let iconClass = $derived.by(() => {
    if (props.progress > 0) return "animate-spin";
    else return "";
  });

  let buttonStyle = $derived.by(() => {
    if (props.progress === 0) return "";

    let angle = (1 - Math.min(1, Math.max(0, props.progress))) * 360;

    return `background-image: conic-gradient(transparent, transparent ${angle}deg, blue ${angle}deg, blue);`;    
  });
</script>

<button class="btn rounded-full" style="{buttonStyle}" onclick={props.onclick}>
  <RefreshCw class={iconClass}/>
</button>

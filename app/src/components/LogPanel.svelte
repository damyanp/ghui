<script lang="ts">
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { Trash2 } from "@lucide/svelte";
  import { tick } from "svelte";

  type Props = {
    onclose: () => void;
  };

  let { onclose }: Props = $props();

  const context = getWorkItemContext();

  let scrollContainer: HTMLDivElement | undefined = $state();
  let panelHeight = $state(200);
  let isDragging = $state(false);

  const logs = $derived(context.logs);

  const MIN_HEIGHT = 80;
  const MAX_HEIGHT_FRACTION = 0.7;

  async function scrollToBottom() {
    await tick();
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  }

  $effect(() => {
    if (logs.length) {
      scrollToBottom();
    }
  });

  function clearLogs() {
    context.logs = [];
    context.markErrorsAsRead();
  }

  function levelClass(level: string): string {
    switch (level) {
      case "error":
        return "text-error-500";
      case "warning":
        return "text-warning-500";
      default:
        return "text-surface-400";
    }
  }

  function levelLabel(level: string): string {
    switch (level) {
      case "error":
        return "ERR";
      case "warning":
        return "WRN";
      default:
        return "INF";
    }
  }

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    isDragging = true;
    const startY = e.clientY;
    const startHeight = panelHeight;

    function onPointerMove(e: PointerEvent) {
      const maxHeight = window.innerHeight * MAX_HEIGHT_FRACTION;
      const newHeight = startHeight + (startY - e.clientY);
      panelHeight = Math.max(MIN_HEIGHT, Math.min(maxHeight, newHeight));
    }

    function onPointerUp() {
      isDragging = false;
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", onPointerUp);
    }

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="h-1.5 cursor-row-resize bg-surface-300-700 hover:bg-primary-500 shrink-0 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary-500"
  class:bg-primary-500={isDragging}
  role="separator"
  tabindex="-1"
  onpointerdown={onPointerDown}
></div>
<div
  class="flex flex-col bg-surface-100-900 shrink-0 overflow-hidden"
  style="height: {panelHeight}px"
>
  <header
    class="flex items-center justify-between gap-2 px-3 py-1 border-b border-surface-300-700 shrink-0"
  >
    <div class="font-bold text-sm">Output ({logs.length})</div>
    <div class="flex items-center gap-1">
      <button
        type="button"
        class="btn p-1"
        title="Clear logs"
        onclick={clearLogs}
      >
        <Trash2 class="size-3.5" />
      </button>
      <button type="button" class="btn p-1 text-xs" onclick={onclose}>
        Close
      </button>
    </div>
  </header>

  {#if logs.length === 0}
    <p class="opacity-70 px-3 py-2 text-sm">No log entries.</p>
  {:else}
    <div
      class="overflow-y-auto flex-1 font-mono text-xs px-3"
      bind:this={scrollContainer}
    >
      {#each logs as entry, i (i)}
        <div class="flex gap-2 py-0.5 border-b border-surface-300-700">
          <span class="opacity-60 shrink-0">{entry.timestamp}</span>
          <span class="shrink-0 w-8 font-semibold {levelClass(entry.level)}"
            >{levelLabel(entry.level)}</span
          >
          <span class="break-all">{entry.message}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

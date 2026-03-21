<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { CircleX, OctagonAlert, Info, Trash2 } from "@lucide/svelte";
  import { tick } from "svelte";

  type Props = {
    open?: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const context = getWorkItemContext();

  let scrollContainer: HTMLDivElement | undefined = $state();

  const logs = $derived(context.logs);

  async function scrollToBottom() {
    await tick();
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  }

  $effect(() => {
    if (open && logs.length) {
      scrollToBottom();
    }
  });

  function onOpenChange(details: { open: boolean }) {
    open = details.open;
    if (!details.open) {
      context.markErrorsAsRead();
    }
  }

  function clearLogs() {
    context.logs.length = 0;
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
</script>

<Modal
  {open}
  contentBase="bg-surface-100-900 p-4 space-y-4 shadow-xl w-[600px] h-screen flex flex-col"
  positionerJustify="justify-end"
  positionerAlign=""
  positionerPadding=""
  transitionsPositionerIn={{ x: 600, duration: 200 }}
  transitionsPositionerOut={{ x: 600, duration: 200 }}
  closeOnEscape={true}
  closeOnInteractOutside={false}
  modal={false}
  onOpenChange={onOpenChange}
>
  {#snippet content()}
    <header class="flex items-center justify-between gap-2">
      <div class="font-bold text-lg">Output Log ({logs.length})</div>
      <div class="flex items-center gap-1">
        <button
          type="button"
          class="btn p-1"
          title="Clear logs"
          onclick={clearLogs}
        >
          <Trash2 class="size-4" />
        </button>
        <button type="button" class="btn p-1" onclick={() => (open = false)}>
          Close
        </button>
      </div>
    </header>

    {#if logs.length === 0}
      <p class="opacity-70">No log entries.</p>
    {:else}
      <div
        class="overflow-y-auto flex-1 font-mono text-xs"
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
  {/snippet}
</Modal>

<script lang="ts">
  import { Dialog, Portal } from "@skeletonlabs/skeleton-svelte";
  import type { Snippet } from "svelte";

  // Local replacement for the monolithic `Modal` component that was removed in
  // @skeletonlabs/skeleton-svelte v4. It composes the new headless Dialog
  // anatomy (Root + Backdrop + Positioner + Content) while keeping the small
  // prop/snippet surface the app relied on.
  type Props = {
    open: boolean;
    onOpenChange?: (details: { open: boolean }) => void;
    modal?: boolean;
    closeOnEscape?: boolean;
    closeOnInteractOutside?: boolean;
    /** Render the dimming backdrop. Disabled for non-modal side panels. */
    backdrop?: boolean;
    /** Classes applied to the content card. */
    contentBase?: string;
    positionerJustify?: string;
    positionerAlign?: string;
    positionerPadding?: string;
    content?: Snippet;
  };

  let {
    open,
    onOpenChange,
    modal = true,
    closeOnEscape = true,
    closeOnInteractOutside = true,
    backdrop = true,
    contentBase = "",
    positionerJustify = "justify-center",
    positionerAlign = "items-center",
    positionerPadding = "p-4",
    content,
  }: Props = $props();
</script>

<Dialog
  {open}
  {modal}
  {closeOnEscape}
  {closeOnInteractOutside}
  {onOpenChange}
>
  <Portal>
    {#if backdrop}
      <Dialog.Backdrop class="fixed inset-0 z-40 bg-surface-950/50" />
    {/if}
    <Dialog.Positioner
      class="fixed inset-0 z-50 flex {positionerJustify} {positionerAlign} {positionerPadding}"
    >
      <Dialog.Content class={contentBase}>
        {@render content?.()}
      </Dialog.Content>
    </Dialog.Positioner>
  </Portal>
</Dialog>

<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, type Icon } from "@lucide/svelte";

  export type MenuItem = {
    icon: typeof Icon;
    label: string;
    iconClass?: string;
    badge?: number;
    disabled?: boolean;
    onclick: () => void;
  };

  type Props = {
    open: boolean;
    onopen: () => void;
    onclose: () => void;
    icon: typeof Icon;
    text: string;
    iconClass?: string;
    disabled?: boolean;
    items: MenuItem[];
  };

  let {
    open,
    onopen,
    onclose,
    icon: TriggerIcon,
    text,
    iconClass,
    disabled = false,
    items,
  }: Props = $props();

  let menuElement = $state<HTMLDivElement | null>(null);
  let triggerButton = $state<HTMLButtonElement | null>(null);

  async function openAndFocusFirst(): Promise<void> {
    onopen();
    await tick();
    const firstItem = menuElement?.querySelector<HTMLElement>(
      '[role="menuitem"]:not([disabled])'
    );
    firstItem?.focus();
  }

  function onButtonKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      onclose();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      void openAndFocusFirst();
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (open) {
        onclose();
      } else {
        void openAndFocusFirst();
      }
    }
  }

  function onMenuKeydown(event: KeyboardEvent): void {
    if (!menuElement) return;
    const menuItems = Array.from(
      menuElement.querySelectorAll<HTMLElement>('[role="menuitem"]:not([disabled])')
    );
    if (menuItems.length === 0) return;
    const currentIndex = menuItems.findIndex(
      (item) => item === document.activeElement
    );
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
      triggerButton?.focus();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      const nextIndex =
        currentIndex < 0 ? 0 : (currentIndex + 1) % menuItems.length;
      menuItems[nextIndex]?.focus();
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      const nextIndex =
        currentIndex < 0
          ? menuItems.length - 1
          : (currentIndex - 1 + menuItems.length) % menuItems.length;
      menuItems[nextIndex]?.focus();
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      menuItems[0]?.focus();
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      menuItems[menuItems.length - 1]?.focus();
    }
  }
</script>

<div class="relative mx-1">
  <button
    bind:this={triggerButton}
    class="btn rounded p-0.5 flex-col relative"
    aria-haspopup="menu"
    aria-expanded={open}
    {disabled}
    onkeydown={onButtonKeydown}
    onclick={() => (open ? onclose() : onopen())}
  >
    <TriggerIcon class={iconClass} />
    <span class="text-xs flex items-center gap-0.5"
      >{text}<ChevronDown size={10} /></span
    >
  </button>

  {#if open}
    <div
      bind:this={menuElement}
      role="menu"
      tabindex="-1"
      class="absolute left-0 top-12 z-50 min-w-44 rounded border border-surface-300-700 bg-surface-100-900 p-1 shadow-lg"
      onkeydown={onMenuKeydown}
    >
      {#each items as item}
        {@const ItemIcon = item.icon}
        <button
          role="menuitem"
          class="btn w-full justify-start gap-2 relative"
          disabled={item.disabled}
          onclick={() => {
            item.onclick();
            onclose();
          }}
        >
          <ItemIcon size={16} class={item.iconClass} />{item.label}
          {#if item.badge !== undefined}
            <span
              aria-label="{item.badge} {item.label}"
              class="ml-auto bg-primary-500 text-white text-[0.6rem] leading-none min-w-3.5 h-3.5 flex items-center justify-center rounded-full px-0.5"
            >{item.badge}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

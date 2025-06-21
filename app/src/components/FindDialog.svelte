<script lang="ts">
  import { Search, SearchSlash } from "@lucide/svelte";
  import { onMount, tick } from "svelte";

  type Props = {
    text: string;
  };

  let { text = $bindable() }: Props = $props();

  let id = $props.id();

  let open = $state(false);

  onMount(() => {
    document.addEventListener("keydown", onkeydown);
    return () => {
      document.removeEventListener("keydown", onkeydown);
    };
  });

  function onkeydown(e: KeyboardEvent) {
    if (!open && e.key.trim().length === 1) {
      text = "";
      open = true;
      tick().then(() => {
        const input = document.getElementById(id);
        input?.focus();
      });
    } else {
      if (e.key === "Escape") {
        text = "";
        open = false;
      }
    }
  }
</script>

{#if open}
  <div class="absolute w-full flex place-content-end pointer-events-none">
    <div
      class="pointer-events-auto dw-1/3 border rounded-4xl z-100 mx-20 m-5 bg-primary-100-900 p-5 flex gap-2 items-center"
    >
      <Search />
      <input
        {id}
        class="flex-1 rounded-lg bg-surface-50-950"
        bind:value={text}
      />
    </div>
  </div>
{/if}

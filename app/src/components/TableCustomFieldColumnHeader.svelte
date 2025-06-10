<script lang="ts">
  import type { Field } from "$lib/bindings/Field";
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import { FileDiff } from "@lucide/svelte";
  import * as octicons from "@primer/octicons";
  import { Popover, Switch } from "@skeletonlabs/skeleton-svelte";
  import type { OpenChangeDetails } from "@zag-js/select";

  type Props = {
    field: Field;
    filter: Array<FieldOptionId | null>;
    onFilterChange: (filter: Array<FieldOptionId | null>) => void;
  };

  const props: Props = $props();
  let filter = $state(props.filter);
  let options = [{ id: null, value: "-" }, ...props.field.options];

  let filterChanged = $state(false);

  const values = $derived.by(() => {
    return options.map((o) => {
      return { ...o, checked: !filter.includes(o.id) };
    });
  });

  function onCheckedChange(id: FieldOptionId | null, checked: boolean) {
    const s = new Set(filter);

    if (checked) {
      s.delete(id);
    } else {
      s.add(id);
    }

    filter = Array.from(s);
    filterChanged = true;
  }

  function onOpenChange(e: OpenChangeDetails) {
    if (!e.open && filterChanged) {
      props.onFilterChange(filter);
    }
  }

  function onAllClicked() {
    filter = []
    filterChanged = true;
  }

  function onNoneClicked() {
    filter = Array.from(options.map(o => o.id));
    filterChanged = true;
  }

  // These octicons are likely useful:
  //
  // filter
  // filter-remove
  // kebab-horizontal
  // sort-asc
  // sort-desc
</script>

<div class="overflow-hidden text-ellipsis w-full">
  {props.field.name}
</div>

<Popover
  arrow
  arrowBackground="!bg-surface-100 dark:!bg-surface-900"
  contentBase="card bg-surface-100-900 p-4 space-y-4 max-w-[320px] "
  {onOpenChange}
>
  {#snippet trigger()}
    <div class="hover:bg-surface-200-800 m-0.5 px-1 rounded">
      {#if filter.length === 0}
      {@html octicons["kebab-horizontal"].toSVG()}
      {:else}
      {@html octicons["filter"].toSVG()}
      {/if}
    </div>
  {/snippet}

  {#snippet content()}
    <div class="grid grid-cols-2">
      <button class="btn preset-tonal m-2 btn-sm" onclick={onAllClicked}>All</button>
      <button class="btn preset-tonal m-2 btn-sm" onclick={onNoneClicked}>None</button>
    </div>
    <div class="grid grid-cols-1">
      {#each values as value}
        <div>
          <Switch
            checked={value.checked}
            onCheckedChange={({ checked }) =>
              onCheckedChange(value.id, checked)}
          />
          {value.value}
        </div>
      {/each}
    </div>
  {/snippet}
</Popover>

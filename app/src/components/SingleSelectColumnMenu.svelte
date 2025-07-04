<script lang="ts">
  import type { Field } from "$lib/bindings/Field";
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { SingleSelect } from "$lib/bindings/SingleSelect";
  import { Switch } from "@skeletonlabs/skeleton-svelte";

  type Props = {
    field: Field<SingleSelect>;
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

  $effect(() => {
    return () => {
      if (filterChanged) {
        props.onFilterChange(filter);
      }
    };
  });

  function onAllClicked() {
    filter = [];
    filterChanged = true;
  }

  function onNoneClicked() {
    filter = Array.from(options.map((o) => o.id));
    filterChanged = true;
  }
</script>

<div class="flex flex-col gap-2 rounded bg-surface-200-800 p-2">
  <div class="text-xs border-b border-surface-500">Filters</div>
  <div class="grid grid-cols-2">
    <button class="btn preset-tonal m-2 btn-sm" onclick={onAllClicked}
      >All</button
    >
    <button class="btn preset-tonal m-2 btn-sm" onclick={onNoneClicked}
      >None</button
    >
  </div>
  <div class="grid grid-cols-1 max-h-[50vh] overflow-y-auto">
    {#each values as value}
      <div>
        <Switch
          checked={value.checked}
          onCheckedChange={({ checked }) => onCheckedChange(value.id, checked)}
        >
          {value.value}
        </Switch>
      </div>
    {/each}
  </div>
</div>

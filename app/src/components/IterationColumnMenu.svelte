<script lang="ts">
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { Fields } from "$lib/bindings/Fields";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import { Switch } from "@skeletonlabs/skeleton-svelte";

  let context = getWorkItemContext();

  type Props = {
    fieldName: keyof Fields;
  };

  const { fieldName }: Props = $props();

  let field = context.getIterationField(fieldName);

  let filter = $state(context.getFilter(fieldName));

  let options = [{ id: null, value: "-", data: undefined }, ...field.options];

  let filterChanged = $state(false);

  let showAll = $state(true);

  const values = $derived.by(() => {
    return options
      .map((o) => {
        return {
          ...o,
          checked: !filter.includes(o.id),
          count: countRowsUsingOption(o.id),
        };
      })
      .filter((o) => showAll || o.count > 0);
  });

  function countRowsUsingOption(optionId: FieldOptionId | null): number {
    const w = Object.values(context.data.workItems);
    return w.filter((w) => {
      if (w) {
        const value = context.getIterationFieldValue(fieldName, w);
        return value === optionId;
      } else {
        return false;
      }
    }).length;
  }

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
        context.setFilter(fieldName, filter);
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
  <div>
    <Switch
      checked={showAll}
      onCheckedChange={(d) => (showAll = d.checked)}
      classes="text-xs"
    >
      {#if showAll}
        Show All
      {:else}
        Show Used
      {/if}
    </Switch>
  </div>
  <div class="border-surface-500 border-t"></div>
  <div class="grid grid-cols-1 max-h-[50vh] overflow-y-auto">
    {#each values as value}
      <div>
        <Switch
          classes="py-1 text-s text-nowrap"
          checked={value.checked}
          onCheckedChange={({ checked }) => onCheckedChange(value.id, checked)}
        >
          {value.value}
          {#if value.data}
            <span class="text-xs text-nowrap">{value.data.startDate}</span>
          {/if}
          {#snippet activeChild()}
            {#if value.count > 0}
              <span class="text-xs">{value.count}</span>
            {/if}
          {/snippet}
        </Switch>
      </div>
    {/each}
  </div>
</div>

<script lang="ts">
  import type { Field } from "$lib/bindings/Field";
  import type { FieldOption } from "$lib/bindings/FieldOption";
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { Fields } from "$lib/bindings/Fields";
  import type { Iteration } from "$lib/bindings/Iteration";
  import type { ProjectItem } from "$lib/bindings/ProjectItem";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
  import * as octicons from "@primer/octicons";
  import { Popover, Switch } from "@skeletonlabs/skeleton-svelte";
  import type { OpenChangeDetails } from "@zag-js/select";

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

  function onOpenChange(e: OpenChangeDetails) {
    if (!e.open && filterChanged) {
      context.setFilter(fieldName, filter);
    }
  }

  function onAllClicked() {
    filter = [];
    filterChanged = true;
  }

  function onNoneClicked() {
    filter = Array.from(options.map((o) => o.id));
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
  {field.name}
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
    <div class="border-b pb-2 grid grid-cols-2">
      <button class="btn preset-tonal m-2 btn-sm" onclick={onAllClicked}
        >All</button
      >
      <button class="btn preset-tonal m-2 btn-sm" onclick={onNoneClicked}
        >None</button
      >
      <Switch checked={showAll} onCheckedChange={(d) => (showAll = d.checked)}>
        {#if showAll}
          Show All
        {:else}
          Show Used
        {/if}
      </Switch>
    </div>
    <div class="grid grid-cols-1 max-h-[50vh] overflow-y-auto">
      {#each values as value}
        <div>
          <Switch
          classes="py-1"
            checked={value.checked}
            onCheckedChange={({ checked }) =>
              onCheckedChange(value.id, checked)}
          >
            {value.value}
            {#if value.data}
              <small>{value.data.startDate}</small>
            {/if}
            {#snippet activeChild()}
              {value.count}
            {/snippet}
          </Switch>
        </div>
      {/each}
    </div>
  {/snippet}
</Popover>

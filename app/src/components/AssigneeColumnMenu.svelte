<script lang="ts">
  import { Switch } from "@skeletonlabs/skeleton-svelte";
  import { untrack } from "svelte";

  type Props = {
    // All assignee logins available across the loaded work items.
    assignees: Array<string>;
    // Current exclusion filter; `null` represents unassigned items.
    filter: Array<string | null>;
    onFilterChange: (filter: Array<string | null>) => void;
  };

  const props: Props = $props();
  // We intentionally capture only the initial value of the parent's filter
  // (this component owns the filter locally until it flushes back via
  // onFilterChange) and of the assignee list (fixed for the menu's lifetime).
  let filter = $state(untrack(() => props.filter));
  // `null` models the "Unassigned" bucket, mirroring the "-" entry in
  // SingleSelectColumnMenu.
  let options = untrack(() => [
    { id: null as string | null, value: "Unassigned" },
    ...props.assignees.map((a) => ({ id: a as string | null, value: `@${a}` })),
  ]);

  let filterChanged = $state(false);

  const values = $derived.by(() => {
    return options.map((o) => {
      return { ...o, checked: !filter.includes(o.id) };
    });
  });

  function onCheckedChange(id: string | null, checked: boolean) {
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

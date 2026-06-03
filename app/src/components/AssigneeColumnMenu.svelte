<script lang="ts">
  import { Switch } from "@skeletonlabs/skeleton-svelte";
  import { untrack } from "svelte";

  type Props = {
    // All assignee logins available across the work items.
    assignees: Array<string>;
    // Logins (plus `null` for unassigned) currently being filtered out.
    filter: Array<string | null>;
    onFilterChange: (filter: Array<string | null>) => void;
  };

  const props: Props = $props();
  // We intentionally capture only the initial value of the parent's filter
  // (this component owns the filter locally until it flushes back via
  // onFilterChange) and of the assignee list (which is fixed for the lifetime
  // of the menu). `null` represents the "unassigned" bucket.
  let filter = $state(untrack(() => props.filter));
  let options = untrack(() => [
    { id: null as string | null, value: "(unassigned)" },
    ...props.assignees.map((login) => ({ id: login, value: `@${login}` })),
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
          class="py-1 text-sm text-nowrap"
          checked={value.checked}
          onCheckedChange={({ checked }) => onCheckedChange(value.id, checked)}
        >
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          <Switch.HiddenInput />
          <Switch.Label>
            {value.value}
          </Switch.Label>
        </Switch>
      </div>
    {/each}
  </div>
</div>

<script lang="ts" generics="T">
  import type { Field } from "$lib/bindings/Field";
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { FieldOption } from "$lib/bindings/FieldOption";
  import * as select from "@zag-js/select";
  import { portal, useMachine, normalizeProps } from "@zag-js/svelte";

  type Props = {
    field: Field<T>;
    defaultValue: FieldOptionId|undefined;
    onValueChange: (value: FieldOptionId | undefined) => void;
  };

  const {
    field,
    defaultValue,
    ...props
  }: Props = $props();

  const options = [{ id: "", value: "-" }, ...field.options];

  const collection = select.collection({
    items: options,
    itemToString: (item) => item.value,
    itemToValue: (item) => item.id,
  });

  const id = $props.id();
  const service = useMachine(select.machine, {
    id,
    collection,
    defaultValue: [defaultValue || ""],
    onValueChange,
  });
  const api = $derived(select.connect(service, normalizeProps));

  function onValueChange(details: select.ValueChangeDetails<FieldOption<T>>) {
    const items = details.items;

    if (items.length === 0) {
      props.onValueChange(undefined);
      return;
    }

    const item = items[0];

    if (item.id === "") props.onValueChange(undefined);
    else props.onValueChange(item.id);
  }
</script>

<div {...api.getRootProps()}>
  <div {...api.getControlProps()}>
    <button
      {...api.getTriggerProps()}
      class="w-full text-left data-[state=open]:bg-primary-50-950"
    >
      {api.valueAsString || "-"}
    </button>
  </div>

  <div use:portal {...api.getPositionerProps()}>
    <ul {...api.getContentProps()} class="bg-surface-50-950 py-3 border">
      {#each options as item}
        <li {...api.getItemProps({ item })} class="w-full">
          <div {...api.getItemTextProps({ item })} class="menu">
            {item.value}
          </div>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style lang="postcss">
  @reference "../app.css";
  .menu {
    @apply cursor-default 
        data-[state=checked]:bg-primary-50-950 data-[state=checked]:text-primary-950-50 
        hover:bg-surface-950-50 hover:text-surface-50-950 
        w-full 
        px-3 py-1;
  }
</style>

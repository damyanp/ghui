// Pure helpers for filterable-field metadata. Kept dependency-free so they can
// be unit tested without the Tauri/Svelte runtime.

import type { Data } from "./bindings/Data";
import type { FieldOptionId } from "./bindings/FieldOptionId";
import type { Filters } from "./bindings/Filters";
import type { WorkItem } from "./bindings/WorkItem";

/** Names of all fields that support filtering, derived from the `Filters`
 * struct. Single source of truth used wherever code needs to enumerate or test
 * for filterable fields. */
export function getFilterableFields(data: Data): Array<keyof Filters> {
  return Object.keys(data.filters) as Array<keyof Filters>;
}

export function isFilterableField(
  data: Data,
  name: string
): name is keyof Filters {
  return name in data.filters;
}

/** Returns the FieldOptionId currently set on `workItem` for any filterable
 * field. The corresponding `projectItem[fieldName]` is either a raw
 * `FieldOptionId | null` or a `DelayLoad`-wrapped one; both are unwrapped here
 * so callers don't have to care. */
export function getFilterableFieldValue(
  workItem: WorkItem,
  fieldName: keyof Filters
): FieldOptionId | null {
  const v = workItem.projectItem[fieldName];
  if (v === null || typeof v === "string") return v;
  return v.loadState === "loaded" ? v.value : null;
}

/** Returns all option ids (including `null` for "unset") for a filterable
 * field. `Field<SingleSelect>` and `Field<Iteration>` share the `options`
 * shape so they can be handled uniformly. */
export function getFilterableFieldOptionIds(
  data: Data,
  fieldName: keyof Filters
): Array<FieldOptionId | null> {
  return [null, ...data.fields[fieldName].options.map((o) => o.id)];
}

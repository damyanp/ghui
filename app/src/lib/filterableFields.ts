// Pure helpers for filterable-field metadata. Kept dependency-free so they can
// be unit tested without the Tauri/Svelte runtime.

import type { Data } from "./bindings/Data";
import type { FieldOptionId } from "./bindings/FieldOptionId";
import type { Filters } from "./bindings/Filters";
import type { WorkItem } from "./bindings/WorkItem";

/** The subset of `Filters` keys that represent option-id-based field filters
 *  (i.e. the per-field exclusion lists). `Filters` also carries entries that
 *  are not option-id lists — the `hideClosed` boolean toggle and the
 *  free-form `assignee` login list; those are not "filterable fields" in the
 *  column-menu sense and are excluded from `FilterableField`. */
export type FilterableField = Exclude<keyof Filters, "hideClosed" | "assignee">;

/** Keys of `Filters` that are NOT per-field option lists. Keep this in sync
 *  with the non-option-id fields on the Rust `Filters` struct
 *  (`ghui-app/src/lib.rs`): the `hideClosed` boolean and the free-form
 *  `assignee` login list. When this drifts, `getFilterableFields` will start
 *  returning a key that callers expect to have `options`, which will explode
 *  at runtime. */
const NON_FIELD_FILTER_KEYS: ReadonlySet<string> = new Set([
  "hideClosed",
  "assignee",
]);

/** Names of all fields that support filtering, derived from the `Filters`
 * struct minus the non-field boolean toggles. Single source of truth used
 * wherever code needs to enumerate or test for filterable fields. */
export function getFilterableFields(data: Data): Array<FilterableField> {
  return Object.keys(data.filters).filter(
    (k) => !NON_FIELD_FILTER_KEYS.has(k)
  ) as Array<FilterableField>;
}

export function isFilterableField(
  data: Data,
  name: string
): name is FilterableField {
  return (
    !NON_FIELD_FILTER_KEYS.has(name) && Object.hasOwn(data.filters, name)
  );
}

/** Returns the FieldOptionId currently set on `workItem` for any filterable
 * field. The corresponding `projectItem[fieldName]` is either a raw
 * `FieldOptionId | null` or a `DelayLoad`-wrapped one; both are unwrapped here
 * so callers don't have to care. Returns `null` when the field is explicitly
 * unset, and `undefined` when a `DelayLoad`-wrapped value has not loaded yet
 * so callers can distinguish "unset" from "unknown" (e.g. to suppress a
 * quick-filter action that would otherwise treat unloaded as `(none)`). */
export function getFilterableFieldValue(
  workItem: WorkItem,
  fieldName: FilterableField
): FieldOptionId | null | undefined {
  const v = workItem.projectItem[fieldName];
  if (v === null || typeof v === "string") return v;
  return v.loadState === "loaded" ? v.value : undefined;
}

/** Returns all option ids (including `null` for "unset") for a filterable
 * field. `Field<SingleSelect>` and `Field<Iteration>` share the `options`
 * shape so they can be handled uniformly. */
export function getFilterableFieldOptionIds(
  data: Data,
  fieldName: FilterableField
): Array<FieldOptionId | null> {
  return [null, ...data.fields[fieldName].options.map((o) => o.id)];
}

/** Returns the GitHub login names assigned to a work item. Draft issues carry
 * no assignees, so an empty array is returned for them. Mirrors the Rust
 * `WorkItem::assignees` accessor. */
export function getAssignees(workItem: WorkItem): Array<string> {
  const data = workItem.data;
  if (data.type === "issue" || data.type === "pullRequest") {
    return data.assignees;
  }
  return [];
}

/** Returns the sorted, de-duplicated set of assignee logins across all work
 * items. Used to populate the assignee filter menu, which (unlike single-select
 * fields) has no fixed option list to draw from. */
export function getAllAssignees(data: Data): Array<string> {
  const seen = new Set<string>();
  for (const id of Object.keys(data.workItems)) {
    const item = data.workItems[id as keyof typeof data.workItems];
    if (item) {
      for (const assignee of getAssignees(item)) {
        seen.add(assignee);
      }
    }
  }
  return Array.from(seen).sort((a, b) => a.localeCompare(b));
}

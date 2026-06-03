// Pure helpers for filterable-field metadata. Kept dependency-free so they can
// be unit tested without the Tauri/Svelte runtime.

import type { Data } from "./bindings/Data";
import type { FieldOptionId } from "./bindings/FieldOptionId";
import type { Filters } from "./bindings/Filters";
import type { WorkItem } from "./bindings/WorkItem";

/** The subset of `Filters` keys that represent option-id-based field filters
 *  (i.e. the per-field exclusion lists). `Filters` also carries boolean
 *  toggles such as `hideClosed`; those are not "filterable fields" in the
 *  column-menu sense and are excluded from `FilterableField`. */
export type FilterableField = Exclude<keyof Filters, "hideClosed" | "assignee">;

/** Keys of `Filters` that are NOT per-field option lists. Keep this in sync
 *  with the boolean fields on the Rust `Filters` struct (`ghui-app/src/lib.rs`).
 *  When this drifts, `getFilterableFields` will start returning a key that
 *  callers expect to have `options`, which will explode at runtime.
 *
 *  `assignee` is excluded because it is a free-form list of logins, not a
 *  single-select field with a fixed `options` list; it has its own dedicated
 *  filter UI (see `AssigneeColumnMenu`). */
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

/** Returns the sorted, de-duplicated set of assignee logins across all work
 * items in `data`. Used to populate the assignee filter menu. Draft issues
 * have no assignees and are skipped. */
export function getAllAssignees(data: Data): Array<string> {
  const logins = new Set<string>();
  for (const workItem of Object.values(data.workItems)) {
    if (!workItem) continue;
    const wd = workItem.data;
    if (wd.type === "issue" || wd.type === "pullRequest") {
      for (const login of wd.assignees) logins.add(login);
    }
  }
  return Array.from(logins).sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: "base" })
  );
}

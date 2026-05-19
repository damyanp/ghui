### 2026-05-19T13:13:15Z: Task 9 (Orthogonal toolbar toggles) shipped — closes pivoting plan
**By:** Linus (for Damyan Pepper)
**What:** Five-toggle toolbar wired in `RecipeBar.svelte`, each on the right state layer: `explodeMulti`/`showGhostAncestors` on `PivotConfig` (existing, also surfaced as checkboxes); new `hideClosed` on `Filters.hide_closed` (Rust, `#[serde(default)]` for cache backcompat, persisted via view_config cache); new `showCounts`/`collapseSingleValue` as `$state` on `WorkItemContext` (frontend-only, intentionally NOT persisted because they are pure render concerns). PR #77, commit `77bcf66`. Closes #68.
**Why:** Final task of pivoting plan Phase 4. Issue specified each toggle's state-ownership explicitly; respected that — the *placement* of state matters more than the wiring.

### 2026-05-19T13:13:15Z: `hide_closed` filter semantics — only filter `Loaded(true)`
**By:** Linus (for Damyan Pepper)
**What:** When `Filters.hide_closed` is on, items are excluded only when `is_closed() == Loaded(true)`. `NotLoaded` items stay visible. Pinned in test `_keeps_items_with_unloaded_state`.
**Why:** DelayLoad invariant: never drop rows based on unknown data — if we don't know the state yet, the user should still see the item. Same principle the rest of the codebase already follows for the other DelayLoad fields.

### 2026-05-19T13:13:15Z: Top-bar "Hide Closed" button rewired to new field
**By:** Linus (for Damyan Pepper)
**What:** The existing top-bar "Hide Closed" button in `+page.svelte` used to hack `filters.status` by including the "Closed" status option id. Now it drives `Filters.hideClosed` directly via the new `setHideClosed` method. Same UI, more correct semantics: catches GitHub state CLOSED/MERGED items even when the project Status field isn't set to "Closed".
**Why:** The RecipeBar's new "Hide closed" checkbox and the top-bar button must reflect the same state. Two UIs writing different fields would have been a UX bug — Brady would have noticed immediately. Toolbar button is no longer disabled when no "Closed" status option exists.

### 2026-05-19T13:13:15Z: `FilterableField` type vs `keyof Filters`
**By:** Linus (for Damyan Pepper)
**What:** Introduced `FilterableField = Exclude<keyof Filters, "hideClosed">` plus runtime `NON_FIELD_FILTER_KEYS = new Set(["hideClosed"])` in `filterableFields.ts`. Used in the narrow methods (`getFilterableFields`, `isFilterableField`, `getFilterableFieldValue`, `getFilterableFieldOptionIds`). `WorkItemContext.getFilter`/`setFilter` keep their original `keyof Fields` signature (with internal `as FilterableField` cast) so existing column-iteration call sites still type-check.
**Why:** Adding more boolean toggles to `Filters` will break option-list iteration unless both the type and the runtime set are updated together. Documented in `filterableFields.ts` and in history.md.

### 2026-05-19T13:13:15Z: Frontend-only render state pattern
**By:** Linus (for Damyan Pepper)
**What:** `showCounts` and `collapseSingleValue` are `$state<boolean>(false)` fields on `WorkItemContext`. They do NOT round-trip through Tauri and do NOT persist across restart. RecipeBar uses `bind:` to read/write them.
**Why:** Pure render concerns. Persistence would require a separate cache file or schema migration — not worth it for two booleans whose default-off state is sensible. If users complain they want them sticky, we can move them to `PivotConfig` later (additive, backcompat-safe).

### 2026-05-19T13:13:15Z: Group-count derivation walks flat node array
**By:** Linus (for Damyan Pepper)
**What:** `WorkItemTree.svelte`'s `groupChildCounts = $derived.by(...)` builds a `Map<id, count>` by walking the flat depth-first `context.data.nodes` array. For each group head, count following `workItem` rows until level drops back to `<= head.level`. `rows` filters out group rows with count===1 when `collapseSingleValue` is on. `renderGroup` snippet appends ` (N)` when `showCounts` is on.
**Why:** No tree structure needed because the node array is already flattened in DFS order — children of a group are contiguous and at strictly greater level. Simple, no recursion. O(n²) worst case but n is small in practice.

### 2026-05-19T13:13:15Z: Pivoting plan complete
**By:** Linus (for Damyan Pepper)
**What:** All 9 pivoting tasks are now shipped or in review. Task 9 was Phase 4's final piece. When PR #77 merges, the entire pivot-recipe + frontend-toolbar feature is done end-to-end.
**Why:** Closing checkpoint for the team.

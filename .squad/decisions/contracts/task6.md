### 2026-05-19T11:27:00-07:00: Task 6 wire-up contract
**By:** Rusty
**Scope:** Phase 3 of pivoting plan — switch the live tree from the hardcoded `NodeBuilder` to `RecipeNodeBuilder` driven by `AppState::pivot_config`, and mount `RecipeBar` in `+page.svelte` so the user can edit the recipe and see the tree re-pivot.

---

## Goals

- Replace `NodeBuilder::new(…).build()` at the single call site in `AppState::refresh()` with `RecipeNodeBuilder::new(…, &self.pivot_config).build()`.
- Remove `NodeBuilder` and migrate or delete its tests once nothing references it.
- Add `setPivotConfig(cfg)` to `WorkItemContext` and mount `<RecipeBar>` in `+page.svelte`, gated by a toolbar toggle button.
- Verify that the default recipe (`Pivot(Epic) → Hierarchy`) produces a node list structurally equivalent to today's hardcoded output.
- `PivotConfig` survives an app restart (already persisted by Task 4; this task verifies the round-trip end-to-end).

## Non-goals

- Ghost-row visual styling (muted CSS, italic) — Task 7.
- Click routing on ghost rows — Task 7.
- The three deferred toggles (show counts, collapse single-valued, hide closed inside RecipeBar) — Task 9.
- Any change to `FilterPanel`, filter UI, or `Filters` plumbing.
- Any change to `WorkItemExecutionTracker` or `WorkItemStatistics` modes.

---

## Pre-done items (no action needed)

**Tauri command registration** — `get_pivot_config`, `set_pivot_config`, `parse_recipe`, and `recipe_to_string` are already in `tauri::generate_handler![…]` in `app/src-tauri/src/lib.rs` (added by PRs #72 and #73). The plan's Task 6 spec says to add them; they are pre-done. **Do not re-add them.** This is a plan deviation to note: the spec's `app/src-tauri/src/lib.rs` bullet is already satisfied.

---

## Interface contract

### Rust side (owner: Basher)

#### Call site switch — `ghui-app/src/lib.rs`

File: `ghui-app/src/lib.rs`, function `AppState::refresh()`, currently line ~254.

**Change:**
```rust
// Before
let nodes =
    NodeBuilder::new(&fields, &work_items, &self.filters, &original_work_items).build();

// After
let nodes =
    RecipeNodeBuilder::new(&fields, &work_items, &self.filters, &original_work_items, &self.pivot_config).build();
```

`RecipeNodeBuilder` is already in scope via `use nodes::*;` (line 34 of `lib.rs`) once `nodes.rs` re-exports it (see below). Remove the `NodeBuilder` use if it's no longer referenced after deletion.

#### `RecipeNodeBuilder` visibility — `ghui-app/src/nodes.rs`

Currently `recipe_builder.rs` is `pub(crate) mod recipe_builder;` and all items inside are `#[allow(dead_code)] pub(crate)`. After the call site switches:

1. Remove `#[allow(dead_code)]` from `RecipeNodeBuilder` struct and its `impl` block in `recipe_builder.rs`.
2. Add to `nodes.rs`:
   ```rust
   pub(crate) use recipe_builder::RecipeNodeBuilder;
   ```
   (or change `pub(crate) mod recipe_builder` → `mod recipe_builder` and `pub(crate) use recipe_builder::*` — either is fine; Basher's call on idiom, but keep it `pub(crate)` — not `pub`.)

#### `NodeBuilder` deletion — `ghui-app/src/nodes.rs`

After the call site is switched, `NodeBuilder` is unreferenced by app code. The only remaining references are the tests in `#[cfg(test)] mod nodebuilder_tests` at the bottom of `nodes.rs`.

**Decision:** Delete `NodeBuilder` and `nodebuilder_tests`. The equivalent coverage now lives in `recipe_builder.rs`'s test suite (Task 2's snapshot tests cover the default recipe, which is what `NodeBuilder` hardcoded). Do **not** convert `nodebuilder_tests` to use `RecipeNodeBuilder` — that would be redundant with Task 2's tests. Delete cleanly.

If Basher finds a test case in `nodebuilder_tests` not already covered by Task 2's snapshots, extract and add it to `recipe_builder.rs`'s tests before deleting.

#### End-to-end test — `ghui-app/src/nodes/recipe_builder.rs` (in `#[cfg(test)] mod tests`)

The plan spec says "extend `recipe_builder_tests.rs`" (there is no separate file — tests live in `recipe_builder.rs`). Add one test:

```rust
#[test]
fn test_recipe_node_builder_non_default_recipe_produces_correct_shape() {
    // Use the existing TestData builder to create a small fixture with known
    // Epic and Workstream values. Apply the recipe:
    //   Pivot(Workstream) → Hierarchy
    // Assert the node list has Group nodes for each distinct Workstream value,
    // and that work items appear under the correct group.
}
```

**This test does NOT need `AppState`.** `AppState::refresh()` is not testable in unit tests (it requires a PAT, network, file I/O). The intent of the plan spec's "exercises `AppState::refresh()` with a non-default recipe" is to verify that `RecipeNodeBuilder` is actually called with the live `pivot_config`. The correct test surface is `RecipeNodeBuilder::build()` directly with a non-default recipe — that is sufficient and consistent with Task 2's test pattern.

**What stays the same:**
- `NodeData`, `Node`, `Filters` structs — do not modify.
- `AppState::set_pivot_config()` / `get_pivot_config()` — do not modify.
- `app/src-tauri/src/lib.rs` — do not modify (pre-done).

**What must NOT change:**
- The `DataUpdate::Data` push in `AppState::refresh()` — the existing `pivot_config: self.pivot_config.clone()` field is already there. Do not remove it.
- `RecipeNodeBuilder`'s public API (`new(…)` / `build()`) — Task 7 and beyond depend on it.

---

### Frontend side (owner: Linus)

#### `setPivotConfig` — `app/src/lib/WorkItemContext.svelte.ts`

Add to the `WorkItemContext` class, adjacent to `setFilter`:

```ts
public setPivotConfig(cfg: PivotConfig): void {
  invoke("set_pivot_config", { cfg });
}
```

Fire-and-forget: the result arrives via the watcher channel as `DataUpdate::Data` containing the re-pivoted nodes and the confirmed `pivotConfig`. No separate `get_pivot_config` call is needed at startup — the first `DataUpdate::Data` from `watch_data` already carries the persisted value.

Import needed: `import type { PivotConfig } from "./bindings/PivotConfig";` (check if already imported — it may not be since `PivotConfig` was previously only used inside the inline `data` initializer type shape).

#### Mount point — `app/src/routes/+page.svelte`

**Where:** Below the `<AppBar>` and above the `<div class="flex flex-col flex-1 …">` content block, rendered only when `recipeBarOpen` is true. This matches the LogPanel pattern (toggled by a state boolean, shown as a panel below the AppBar).

**State:**
```ts
let recipeBarOpen = $state(false);
```

**AppBar button** (inside `{#snippet lead()}`, grouped with the existing control buttons — add a `<div class="w-3">` gap before it if needed):
```svelte
<AppBarButton
  icon={ChartNetwork}
  text="Recipe"
  active={recipeBarOpen}
  disabled={disabled}
  onclick={() => { recipeBarOpen = !recipeBarOpen; }}
/>
```

Use a lucide icon — `ChartNetwork`, `Network`, `GitBranch`, or `LayoutTree` from `@lucide/svelte`. Pick the one that renders most clearly at toolbar size. **Do not add a new dependency** — choose from icons already imported or available in `@lucide/svelte`.

**Mounting:**
```svelte
{#if recipeBarOpen}
  <div class="border-b border-surface-300-700 px-4 py-2">
    <RecipeBar
      bind:value={context.data.pivotConfig}
      onApply={(cfg) => context.setPivotConfig(cfg)}
    />
  </div>
{/if}
```

Place this block immediately after the `</AppBar>` closing tag, before `<ReviewChangesPanel>`.

**Import:**
```ts
import RecipeBar from "../components/RecipeBar.svelte";
```

#### Data flow (explicit)

1. User edits recipe text in `RecipeBar`, presses Apply.
2. `RecipeBar.applyCurrentText()` awaits `parseRecipe(text)` (Tauri `parse_recipe` command) → on success calls `onApply(cfg)`.
3. `onApply` = `(cfg) => context.setPivotConfig(cfg)` → fires `invoke("set_pivot_config", { cfg })`.
4. Rust: `AppState::set_pivot_config(cfg)` stores the new config, persists to `view_config.ghui.json`, calls `self.refresh(false).await`.
5. `refresh(false)` builds nodes via `RecipeNodeBuilder::new(…, &self.pivot_config).build()`, emits `DataUpdate::Data(Box::new(Data { nodes, pivot_config, … }))`.
6. Frontend: `on_data_update` → `onDataUpdateData(data)` → `this.data = data`.
7. `context.data.pivotConfig` updates reactively → `RecipeBar`'s `bind:value` receives the canonical (Rust-formatted) value.
8. Tree re-renders from `context.data.nodes` — no separate re-fetch, no separate `refresh()` call from the frontend.

#### State ownership

| State | Lives in | Notes |
|-------|----------|-------|
| `pivotConfig` (canonical) | `AppState` (Rust) | Persisted in `view_config.ghui.json` |
| `context.data.pivotConfig` | `WorkItemContext.data` ($state) | Shadow copy; updated via watcher |
| `recipeBarOpen` | `+page.svelte` local | UI-only toggle |
| `recipeText` (in-flight text) | `RecipeBar.svelte` local | Not persisted; reset to formatted config on each `DataUpdate` |

#### Test surface (frontend)

- **No new unit tests required** for `setPivotConfig` — it is a one-line `invoke` wrapper with no extractable logic. Following the project pattern: only extract to a testable helper when there is logic to test.
- **Existing `RecipeBar.test.ts`** — no changes needed; component behavior is already tested in isolation.
- **Smoke check:** `cd app && npm run check` must pass with the new `recipeBarOpen` state, `RecipeBar` import, and `setPivotConfig` method.

---

## Shared invariants (both sides enforce)

1. **Empty recipe → empty tree, not a crash.** `RecipeNodeBuilder::build()` with `recipe: []` must return an empty `Vec<Node>` (or a flat sorted list of all items, per `render_scope`'s base case). Verify this does not panic.
2. **Default recipe equivalence.** The default `PivotConfig` (`Pivot(Epic) → Hierarchy`) must produce a node list structurally equivalent to what `NodeBuilder::add_nodes()` produced today for any project where every child's Epic agrees with its parent's. This is the primary regression gate.
3. **No partial state.** The tree re-render is atomic — `DataUpdate::Data` carries both the new `nodes` and the confirmed `pivotConfig` together. The frontend never shows a tree built from a different config than the one reflected in `RecipeBar.value`.
4. **`set_pivot_config` does not force a GitHub API call.** It calls `refresh(false)` which uses cached `work_items`/`fields`. Switching recipes is a local operation.
5. **`recipeBarOpen` is UI state only.** Closing the RecipeBar panel does not reset the recipe. The recipe persists in AppState regardless of the panel's open/closed state.

---

## Test gates (CI must pass before merge)

1. `cargo fmt --all -- --check`
2. `cargo clippy --all -- -D warnings`
3. `cargo test --all` (regenerates ts-rs bindings; verify `git diff` shows no unexpected binding changes)
4. `cd app && npm run check`
5. `cd app && npm test`

**Task-specific gates:**
- The new `test_recipe_node_builder_non_default_recipe_produces_correct_shape` test must pass and cover at least one non-default recipe with ≥ 2 distinct bucket values.
- Manual smoke: `cd app && npx tauri dev` — default tree renders, RecipeBar toggles open/closed, switching to a preset re-renders the tree without GitHub fetch, restart preserves the chosen recipe.
- **Regression gate:** Default recipe (`Pivot(Epic) → Hierarchy`) must produce the same visual grouping as the pre-Task-6 tree for a known project. Document the comparison in the PR description (screenshot or node-list comparison from logs).

---

## Plan deviation notes (for the Scribe to amend)

1. **`app/src-tauri/src/lib.rs` is pre-done.** The plan's Task 6 bullet "register `get_pivot_config`, `set_pivot_config` in `tauri::generate_handler!`" is already satisfied by PR #72. The plan should note this was done early.
2. **"End-to-end test exercises `AppState::refresh()`" is a misnomer.** `AppState` is not unit-testable (requires PAT + network). The correct test is `RecipeNodeBuilder::build()` with a non-default recipe against a fixture dataset. The plan spec should say "add a `RecipeNodeBuilder` integration test with a non-default recipe" rather than reference `AppState::refresh()`.
3. **"`ghui-app/src/nodes/recipe_builder_tests.rs`" does not exist as a separate file.** Task 2 placed tests in the `#[cfg(test)] mod tests` block inside `recipe_builder.rs` (documented in decisions.md PR #70 verdict). The Task 6 spec references extending that file by its originally planned name. The correct action is to add to the existing `#[cfg(test)] mod tests` in `recipe_builder.rs`.

---

## Implementation order (parallel split)

**Basher (Rust — fully independent of Linus):**
1. Remove `#[allow(dead_code)]` from `RecipeNodeBuilder` in `recipe_builder.rs`.
2. Add `pub(crate) use recipe_builder::RecipeNodeBuilder;` to `nodes.rs`.
3. Switch call site in `AppState::refresh()` from `NodeBuilder` to `RecipeNodeBuilder`.
4. Delete `NodeBuilder` struct, `NodeBuilder` `impl`, and `mod nodebuilder_tests` from `nodes.rs`. Verify no remaining references.
5. Add the `test_recipe_node_builder_non_default_recipe_produces_correct_shape` test.
6. Run all 5 validation commands. Fix any clippy/fmt issues.

**Linus (Frontend — fully independent of Basher):**
1. Add `setPivotConfig(cfg: PivotConfig): void` to `WorkItemContext.svelte.ts`.
2. Add `recipeBarOpen = $state(false)` and the AppBarButton in `+page.svelte`.
3. Mount `<RecipeBar bind:value={context.data.pivotConfig} onApply={…} />` below the AppBar.
4. Run `npm run check` and `npm test`. Fix any type errors.

**Convergence (either agent, after both branches commit):**
- Merge both branches. Run `cargo test --all` to confirm no binding changes (there should be none — no new ts-rs exports in this task).
- If the two branches are on the same PR branch (`pivoting/task6-wire-up`), coordinate commit order to avoid conflicts. The files touched are disjoint: Basher touches `ghui-app/src/`, Linus touches `app/src/`.

---

## Risks / open questions

1. **Default recipe equivalence.** The old `NodeBuilder` sorted by `fields.epic.option_index(key)`. `RecipeNodeBuilder`'s `Pivot(Epic)` bucket ordering must match. Basher should verify by comparing node-ID sequences against a known project snapshot before declaring the switch done. If there's a mismatch, it is a blocker — do not merge.

2. **`RecipeBar` layout in the AppBar.** RecipeBar is a multi-line panel (~4 rows: text input, preset dropdown, toggles, error/grammar). Mounting it inline in the AppBar's `{#snippet lead()}` will not work — the AppBar is a single horizontal strip. The contract specifies mounting it as a panel below the AppBar (toggled by a button). If Damyan prefers a different UX (e.g., a modal dialog, a side panel, or always-visible below AppBar without toggle), **escalate before implementing** — the mount point affects the layout significantly.

   **Rusty's recommendation:** Toggled panel below AppBar, as specified in this contract. It matches the existing `LogPanel` pattern. Low risk.

3. **`RecipeBar` async parse on Apply vs. on typing.** Currently `RecipeBar` only parses on Apply (button click or Enter). The tree does not re-render while the user types. This is correct behavior — do not change it.

4. **`NodeBuilder` test coverage gap.** If `nodebuilder_tests` contains any test case not replicated by Task 2's snapshot suite, deleting it loses coverage. Basher must audit before deleting. If there's a gap, add a targeted test to `recipe_builder.rs` first.

5. **`RecipeNodeBuilder::build()` with an empty `work_items` set.** `render_scope` on an empty item list returns early. Confirm no panic path exists if `Fields` has no options for a bucketed field (e.g., `Workstream` field has zero options). This is an edge case in the existing `bucket_by_field` logic — Basher should verify.

---

## Branch & PR plan

- **Branch:** `pivoting/task6-wire-up` (NOT a `copilot/*` branch — joint authorship)
- **Target:** `main`
- **PR title:** `pivoting(task6): wire RecipeNodeBuilder + RecipeBar end-to-end`
- **Joint authorship:** every commit must carry `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` trailers for both Basher and Linus
- **PR description must include:** explicit pass/fail for all 5 validation commands, a screenshot or node-list comparison demonstrating default-recipe equivalence

# Pivoting — Implementation Plan

> Companion to [`pivoting-design.md`](./pivoting-design.md). The
> design doc explains *what* we are building (Option D: composable
> view recipes); this doc explains *how* we slice the work so that
> several cloud agents can make progress in parallel without
> stepping on each other.
>
> A working prototype of the end-state is checked in at
> [`pivoting-prototype.html`](./pivoting-prototype.html); each task
> below references the part of the prototype it is replacing in the
> real app.

## Scope

Replace the hardcoded `Pivot(Epic) → Hierarchy` grouping in
[`ghui-app/src/nodes.rs`](../ghui-app/src/nodes.rs) (`NodeBuilder`)
with a configurable `Vec<Axis>` recipe driven from the toolbar by a
textual recipe input + curated preset dropdown, persisted alongside
`Filters` in `AppState`.

Out of scope for this plan (deferred follow-ups):

- Chip-style visual recipe builder (textual input is iteration 1
  per design §11).
- Statistics-view changes (§3 N3).
- Drag-and-drop semantics on ghost rows beyond "click routes to
  primary".
- Saved-recipe naming / favourites UI.

## Phasing overview

```
   ┌──────────────┐
   │ Phase 1      │   Foundation. Single agent. Must land first.
   │ Task 1       │   No callers, pure types + parser.
   └───────┬──────┘
           │
   ┌───────┴───────────────────────────────────────────┐
   │ Phase 2  (parallel — 4 cloud agents)              │
   │   Task 2: recipe-aware node builder (Rust)        │
   │   Task 3: RecipeBar.svelte + presets (frontend)   │
   │   Task 4: PivotConfig in AppState + Tauri cmds    │
   │   Task 5: TS recipe parser parity (frontend)      │
   └───────┬───────────────────────────────────────────┘
           │
   ┌───────┴──────┐
   │ Phase 3      │   Wire-up. Single agent. Flips behaviour.
   │ Task 6       │   Depends on all of Phase 2.
   └───────┬──────┘
           │
   ┌───────┴───────────────────────────────────────────┐
   │ Phase 4  (parallel polish — 3 cloud agents)       │
   │   Task 7: ghost-row visuals + click routing       │
   │   Task 8: multi-value (Assignee) Explode toggle   │
   │   Task 9: orthogonal toolbar toggles              │
   └───────────────────────────────────────────────────┘
```

## Coordination rules

These rules let several agents work in parallel without merge
conflicts:

1. **Phase 2 tasks are additive only.** They must not modify
   `NodeBuilder::add_nodes()` or
   [`WorkItemTree.svelte`](../app/src/components/WorkItemTree.svelte)
   rendering logic — those are reserved for Task 6.
2. **Each Phase 2 task ships its own tests** that pass independently
   without the other Phase 2 tasks merged.
3. **All five validation commands must pass** before opening a PR
   (per [`copilot-instructions.md`](../.github/copilot-instructions.md#do)):
   `cargo fmt --all -- --check`, `cargo clippy --all -- -D warnings`,
   `cargo test --all`, `cd app && npm run check`, `cd app && npm test`.
4. **Recipe fixture file is the contract** between Tasks 2 and 5.
   Task 1 produces `github-graphql/tests/fixtures/recipes.json`
   (canonical recipe → expected parse tree); Tasks 2 and 5 both
   consume it so the Rust and TS parsers agree exactly.
5. **PR titles follow `pivoting(taskN): …`** so the PR list groups
   them.

---

## Phase 1 — Foundation

### Task 1: Recipe types, parser, and ts-rs export

**One agent. Blocks Phase 2. Small, well-scoped.**

**New files:**
- `github-graphql/src/pivot.rs`
- `github-graphql/tests/fixtures/recipes.json`

**Modified files:**
- `github-graphql/src/lib.rs` (add `pub mod pivot;`)

**Types to add (all `#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]`,
`#[ts(export)]`, `#[serde(rename_all = "camelCase")]`):**

```rust
pub enum PivotField {
    Status, Blocked, Epic, Iteration, Kind, Workstream,
    Estimate, Priority, Assignee, Repository, Type, State,
}

pub enum Axis {
    Pivot(PivotField),
    Group(PivotField),
    Hierarchy,
    Sort(PivotField),
}

pub enum MultiValueStrategy { Combined, Explode }

pub struct PivotConfig {
    pub recipe: Vec<Axis>,
    pub multi_value_strategy: MultiValueStrategy,
    pub show_ghost_ancestors: bool,
}

impl Default for PivotConfig {
    fn default() -> Self {
        Self {
            recipe: vec![Axis::Pivot(PivotField::Epic), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        }
    }
}
```

**Parser API:**

```rust
pub fn parse_recipe(text: &str) -> anyhow::Result<Vec<Axis>>;
pub fn recipe_to_string(recipe: &[Axis]) -> String;
```

**Grammar (must match the prototype's `parseRecipe()` exactly):**
```
recipe := axis (SEP axis)*
SEP    := "→" | "->" | ">" | ","
axis   := "Pivot" "(" field ")"
        | "Group" "(" field ")"
        | "Sort"  "(" field ")"
        | "Hierarchy"
field  := <case-insensitive enum name with aliases>
```

Field aliases: `Repo → Repository`, `Owner → Assignee`,
`Assignees → Assignee`. (Pull the full alias list from
`FIELD_ALIASES` in the prototype HTML.)

**Tests:**

- Round-trip every preset from the prototype
  (`PRESETS` in `pivoting-prototype.html`) — parse then stringify
  and assert equality up to canonical separator.
- Error cases: unknown field, unknown axis, missing parens.
- Snapshot test of the fixtures file (use `insta`).

**Acceptance:**
- `cargo test -p github-graphql` passes.
- `cargo test --all` regenerates ts-rs bindings for `Axis`,
  `PivotField`, `PivotConfig`, `MultiValueStrategy` cleanly.
- `cargo clippy --all -- -D warnings` clean.
- All five validation commands pass.

**Out of scope for this task:** any callers, AppState plumbing, UI.
The new module exists in isolation.

---

## Phase 2 — Parallel implementation

All four tasks branch off the commit that lands Task 1 and can be
worked on simultaneously.

### Task 2: Recipe-aware node builder

**One agent. Independent of Tasks 3/4/5.**

**New files:**
- `ghui-app/src/nodes/recipe_builder.rs`

**Modified files:**
- `ghui-app/src/nodes.rs` (turn into a `mod nodes { … }` directory
  module; existing `NodeBuilder` keeps working unchanged).

**API:**

```rust
pub(crate) struct RecipeNodeBuilder<'a> { /* same context as NodeBuilder */ }

impl<'a> RecipeNodeBuilder<'a> {
    pub fn new(/* same as NodeBuilder + */ pivot_config: &'a PivotConfig) -> Self;
    pub fn build(&mut self) -> Vec<Node>;
}
```

**Behaviour to implement (mirrors prototype `renderScope` /
`renderHierarchy` / `renderTreeNode`):**

1. Recursively interpret the axis list:
   - `Pivot(field)` / `Group(field)` → bucket the current scope by
     that field's value (or sorted set of values for multi-valued
     fields under `Combined`); recurse into each bucket with the
     remaining axes.
   - `Hierarchy` → render the source parent/child tree of the
     items in scope; if `show_ghost_ancestors` and a remaining
     axis follows, build the primary + ghost membership per
     §6.4a rule 1, then re-apply the remaining axes to each
     level's children.
   - `Sort(field)` → sort the current scope by that field; no
     bucketing, no level change.
2. Multi-value fields under `Explode` produce one bucket per
   constituent value; ghost membership is computed per assignee
   bucket independently.
3. `Node`'s existing `level: u32` field tracks visual indentation;
   bump it on each `Pivot` / `Group` / hierarchy level.
4. Group `Node`s carry a stable `id` derived from
   `path/<axis-name>=<value>` so the frontend can collapse / expand
   independently of the recipe text.

**Tests (`ghui-app/src/nodes/recipe_builder_tests.rs`):**

- For each preset in `recipes.json`, build against a small fixture
  dataset and snapshot the resulting node sequence.
- Ghost rendering: a parent with mixed-Epic children produces the
  expected ghost rows in each Epic bucket.
- Multi-value Combined vs Explode: an item with two assignees
  produces one synthetic bucket vs two buckets respectively.
- `show_ghost_ancestors = false` flattens the ghosts (parents only
  appear in their own bucket).

**Acceptance:**
- `cargo test --all` passes.
- All five validation commands pass.
- `NodeBuilder` (the old one) is **not** modified or wired in. The
  new builder is unused by the app.

---

### Task 3: `RecipeBar.svelte` + presets (UI shell)

**One agent. Independent of Tasks 2/4/5. Pure frontend.**

**New files:**
- `app/src/components/RecipeBar.svelte`
- `app/src/lib/recipePresets.ts`
- `app/src/lib/recipeText.ts` (string ↔ `PivotConfig` helpers
  that delegate to Task 5's parser; provide a stub initially)
- `app/src/lib/recipeBar.test.ts`
- `app/src/routes/dev/recipe-bar/+page.svelte` (standalone demo)

**Component API:**

```svelte
<script lang="ts">
  import type { PivotConfig } from "$lib/bindings/PivotConfig";
  let { value = $bindable<PivotConfig>(), onApply }: {
    value: PivotConfig;
    onApply: (cfg: PivotConfig) => void;
  } = $props();
</script>
```

**UI elements (mirror the prototype toolbar):**

1. Text input with current recipe; `Apply` button (or Enter).
2. Preset dropdown that writes into the text input.
3. Collapsible grammar help.
4. Inline error row when the text fails to parse.
5. Three to five orthogonal checkbox toggles
   (counts / collapse / hide closed / explode / show ghosts) bound
   into `value`.

**Presets (`recipePresets.ts`):** port the 14-entry `PRESETS` array
from the prototype, exported as
`export const PRESETS: ReadonlyArray<{ label: string; recipe: string }> = […]`.

**Tests (`recipeBar.test.ts`, vitest):**

- `recipeText.parse()` round-trips each preset.
- Picking a preset updates `value.recipe` to the parsed shape.
- Toggle changes are emitted via `onApply` with the updated
  `PivotConfig`.

**Standalone demo route (`/dev/recipe-bar`)** lets a reviewer
drive the component without the rest of the app being wired up.

**Acceptance:**
- `cd app && npm test` and `cd app && npm run check` pass.
- Demo route renders standalone (`npx vite dev`, navigate to
  `/dev/recipe-bar`).
- Component is **not** mounted in the main toolbar yet.

---

### Task 4: `PivotConfig` in `AppState` + Tauri commands

**One agent. Independent of Tasks 2/3/5.**

**Modified files:**
- `ghui-app/src/lib.rs` — add `pivot_config: PivotConfig` next to
  `filters: Filters` in `AppState`; default to
  `PivotConfig::default()`.
- `app/src-tauri/src/lib.rs` — register two new Tauri commands.

**New commands (mirror `set_filters` / `get_filters` patterns):**

```rust
#[tauri::command]
pub async fn get_pivot_config(state: State<'_, DataState>)
    -> TauriCommandResult<PivotConfig>;

#[tauri::command]
pub async fn set_pivot_config(state: State<'_, DataState>, cfg: PivotConfig)
    -> TauriCommandResult<()>;
```

`set_pivot_config` should persist to the same per-project cache file
that `Filters` already writes to (`~/{name}.ghui.json`); reuse the
existing serialization path so a project that already has filters
gains a `pivotConfig` key.

**Tests:**
- Round-trip: write a `PivotConfig` to the cache file, read it
  back, assert equality. Use `tempfile`.
- `AppState::set_pivot_config()` triggers the existing watcher with
  a `DataUpdate` so the frontend can react (mirror what
  `set_filters` does).

**Acceptance:**
- `cargo test --all` passes; ts-rs bindings include
  `PivotConfig`.
- `cargo clippy --all -- -D warnings` clean.
- All five validation commands pass.
- Tree rendering still uses the old `NodeBuilder` and is
  unchanged for the user.

---

### Task 5: TS recipe parser parity

**One agent. Independent of Tasks 2/3/4. Pure frontend logic.**

**New files:**
- `app/src/lib/recipeParser.ts`
- `app/src/lib/recipeParser.test.ts`

**API (must match the Rust parser from Task 1):**

```ts
import type { Axis } from "$lib/bindings/Axis";
export function parseRecipe(text: string): { ok: true; recipe: Axis[] }
                                         | { ok: false; error: string };
export function recipeToString(recipe: Axis[]): string;
```

**Tests:** load `github-graphql/tests/fixtures/recipes.json` (Task 1)
via `import.meta.glob` or by copying it to `app/test-fixtures/`
and assert the same parse tree for every fixture. This makes the
Rust and TS parsers a tested pair, not parallel best-effort
implementations.

**Acceptance:**
- `cd app && npm test` passes.
- Diffing the Rust and TS test outputs against the same fixture
  shows zero divergence.

---

## Phase 3 — Wire-up

### Task 6: Replace hardcoded grouping in the tree

**One agent. Depends on Tasks 1–5 having merged.**

**Modified files:**
- `ghui-app/src/lib.rs` — switch the `NodeBuilder` call site to
  `RecipeNodeBuilder` driven by `self.pivot_config`.
- `ghui-app/src/nodes.rs` — `pub(crate) use
  recipe_builder::RecipeNodeBuilder;`; delete the old
  `NodeBuilder` once the new one is in.
- `app/src/lib/WorkItemContext.svelte.ts` — expose a setter that
  calls `invoke("set_pivot_config", …)`.
- `app/src/routes/+page.svelte` — mount `<RecipeBar bind:value=
  {context.data.pivotConfig} onApply={…} />` behind a toolbar
  toggle (LogPanel pattern), below `</AppBar>`.
- `ghui-app/src/nodes/recipe_builder.rs` — add a unit test
  inside the existing `#[cfg(test)] mod tests` that exercises
  `RecipeNodeBuilder::build()` directly with a non-default
  recipe (driving `AppState::refresh()` from a test is not
  practical — there is no in-memory `AppState` constructor —
  and is not the right gate anyway).

**Plan deviations resolved in PR #74 (kept here for reference):**
1. **Tauri command registration:** Already done by Task 4 (PR
   #72). Not part of Task 6.
2. **End-to-end test target:** The original spec said "exercises
   `AppState::refresh()`." That's a misnomer — the correct test
   target is `RecipeNodeBuilder::build()` directly. Refresh just
   calls into the builder.
3. **`recipe_builder_tests.rs` standalone file:** Does not exist.
   Tests live in `recipe_builder.rs`'s `#[cfg(test)] mod tests`.

See `.squad/decisions/contracts/task6.md` for the full Task 6
design contract written by the reviewer before split.

**Acceptance:**
- App runs (`cd app && npx tauri dev`).
- Default recipe `Pivot(Epic) → Hierarchy` matches today's tree
  byte-for-byte for a project where every child agrees with its
  parent's Epic.
- Switching to `Hierarchy → Group(Workstream)` re-renders without
  fetching from GitHub.
- All five validation commands pass.

**Risk:** this is the only PR that breaks the
"app behaves the same as before" property. Land it deliberately,
not as a drive-by.

---

## Phase 4 — Polish (parallelizable)

Each of these can be done by a separate agent after Task 6 lands.

### Task 7: Ghost-row visuals + click routing

**Modified files:**
- `app/src/components/WorkItemTree.svelte` — apply muted CSS class
  (italic, lower-contrast text colour, e.g. Skeleton's
  `text-surface-500-500`) when `node.isGhost === true`; suppress
  hover affordances.
- `app/src/components/TreeTableContextMenu.svelte` — disable
  edit / drag actions on ghost rows.
- `WorkItemTree.svelte` click handler — if the row is a ghost,
  scroll to and select the primary occurrence of the same
  `WorkItemId` in the current tree.

**Note:** `Node` needs a new boolean `is_ghost: bool` produced by
`RecipeNodeBuilder` (Task 2 should add this to the `Node` struct
already — confirm before starting Task 7).

**Acceptance:** visual review against
`pivoting-prototype.html` with recipe
`Pivot(Status) → Hierarchy`.

### Task 8: Multi-value (Assignee) Explode toggle

**Modified files:**
- `app/src/components/RecipeBar.svelte` — add the Explode checkbox
  if Task 3 left it as a stub.
- `RecipeNodeBuilder` — verify the `Explode` strategy was actually
  implemented in Task 2; add a missing branch if not.

**Acceptance:** with recipe `Pivot(Assignee)`, toggling
"Explode multi-valued" between off and on switches between
synthetic-set buckets and per-assignee buckets.

### Task 9: Orthogonal toolbar toggles

**Modified files:**
- `app/src/components/RecipeBar.svelte` (or split out into
  `RecipeToggles.svelte` if it gets cluttered)

**Toggles to wire (some may already be in Task 3):**
- *Show counts* on group headers.
- *Collapse single-value groups* — buckets with one item render
  inline.
- *Hide closed* — filter out items where `state === "Closed"`
  before bucketing.
- *Show ghost ancestors* — already in `PivotConfig`, just expose.

`hideClosed` is a *filter*, not a recipe axis — it lives next to
`Filters`, not in `PivotConfig::recipe`.

**Acceptance:** each toggle has a one-line vitest assertion in
`recipeBar.test.ts`.

---

## Tracking

When opening a PR for one of these tasks, link back to this doc
and reference the task number:

```
Implements **Task N** from
[`docs/pivoting-implementation-plan.md`](../docs/pivoting-implementation-plan.md).
```

Use `pivoting(taskN): …` as the PR title prefix so the PR list
groups them.

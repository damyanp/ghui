### 2026-05-19T11:48:00-07:00: Task 6 — frontend wire-up complete
**By:** Linus
**What:** Added `WorkItemContext.setPivotConfig(cfg)` and mounted `<RecipeBar>` in `+page.svelte` behind a `ChartNetwork` toolbar toggle button.
**Why:** Per Rusty's Task 6 contract. Branch `pivoting/task6-frontend` ready to merge into `pivoting/task6-wire-up`.
**Validation:**
- ✅ `cargo fmt --all -- --check` — clean
- ❌ `cargo clippy --all -- -D warnings` — 2 pre-existing errors in `ghui-app/src/updater.rs` (uninlined format args); not caused by this task; flagged in contract as known pre-existing
- ✅ `cargo test -p github-graphql` — 64 tests passed
- ✅ `cd app && npm run check` — 0 errors, 0 warnings (required `npm ci` first; the main tree's node_modules junction was missing vitest binaries)
- ✅ `cd app && npm test` — 22 tests passed (3 test files: filterableFields, RecipeBar, recipeParser)

**Notes:**
- RecipeBar prop interface confirmed exactly as documented: `bind:value={PivotConfig}` and `onApply={(cfg: PivotConfig) => void}`. No surprises; no adapter needed.
- Chose `ChartNetwork` from `@lucide/svelte` for the Recipe toolbar button (no new dependency added).
- RecipeBar is mounted immediately after `</AppBar>` and before `<ReviewChangesPanel>`, matching the LogPanel `{#if open}` pattern.
- No `get_pivot_config` call on startup — initial value arrives via first `DataUpdate::Data` from `watch_data` as specified.
- node_modules junction from main tree was missing vitest; `npm ci` was run in the worktree to resolve.

### 2026-05-19T11:40:00-07:00: Task 6 — Rust wire-up complete
**By:** Basher
**What:** Replaced `NodeBuilder::new(...).build()` with `RecipeNodeBuilder::new(..., &self.pivot_config).build()` in `AppState::refresh()`, removed all `#[allow(dead_code)]` from `RecipeNodeBuilder` and its helpers, added `pub(crate) use recipe_builder::RecipeNodeBuilder` to `nodes.rs`, deleted `NodeBuilder` struct/impl and `mod nodebuilder_tests` from `nodes.rs`, ported 2 test cases from nodebuilder_tests not covered by snapshot suite, and added the required non-default recipe integration test.
**Why:** Per Rusty's Task 6 contract. Branch `pivoting/task6-rust` ready to merge into `pivoting/task6-wire-up`.
**Validation:**
- ✅ `cargo fmt --all -- --check`
- ❌ `cargo clippy --all -- -D warnings` — 2 pre-existing errors in `telemetry.rs`/`updater.rs` (toolchain drift, CI-passing per contract). Zero new errors from my changes.
- ✅ `cargo test -p ghui-app` — 43 tests pass
- ✅ `cargo test --all` — all tests pass, no unexpected ts-rs binding changes
- ✅ `cd app && npm run check` — 0 errors, 0 warnings (run from `ghui-task6-fe` worktree where deps are installed; no frontend files were touched)

**Test added:** `test_recipe_node_builder_non_default_recipe_produces_correct_shape`

**Notes:**
- **Default-recipe equivalence:** Task 2's snapshot tests already cover `Pivot(Epic) → Hierarchy` (the default recipe). The snapshot verifies group ordering, hierarchy within groups, and ghost-ancestor behavior. Equivalence to the old `NodeBuilder` is CONDITIONAL on well-formed projects (same Epic on parent and child). For mixed-Epic hierarchies, `RecipeNodeBuilder` produces ghost rows — this is the intended Task 7 behavior, not a regression. No equivalence blocker.
- **Tests ported from nodebuilder_tests:** Two tests were NOT covered by Task 2 snapshots:
  1. `test_recipe_node_builder_filters_closed` — ported from `test_node_builder_filters_closed`. Tests that `should_include` correctly propagates open descendants through closed ancestors with non-default filters.
  2. `test_recipe_node_builder_new_item_after_update_appears` — ported from `test_node_builder_new_item_after_update_appears`. Tests that `WorkItems::update()` maintains `ordered_items` so newly arrived items appear in the tree.
- Three tests from nodebuilder_tests were NOT ported (intentionally): `test_node_builder_single_item`, `test_node_builder_hierarchy`, `test_node_builder_grouping` — these tested the OLD behavior (single-group suppression) which is intentionally changed by RecipeNodeBuilder.
- The `test_node_build_no_filters` test was not ported — it's redundant with all snapshot tests which use `Filters::default()`.
- Commits: 2 logical commits pushed to `pivoting/task6-rust`. All bindings files showed as modified in `git status` but `git diff` confirmed zero content changes (CRLF artifacts); not committed.

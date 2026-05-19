### 2026-05-19: Task 8 (Multi-value Explode toggle) — coverage-only PR #75
**By:** Linus (Frontend Dev)
**What:** Shipped Task 8 of the pivoting plan as PR #75
(`pivoting/task8-explode-toggle` → `main`, Closes #67). Three files changed
(+121 / −6): extracted a pure `getToggleChecked(config, toggle)` helper into
`app/src/components/recipeBarState.ts` (mirrors `setToggle`), updated
`RecipeBar.svelte` to consume it (no behaviour change — the inline
`checked: (config) => …` closures in `liveToggles` are gone), and added
nine vitest cases in two new `describe` blocks covering the three issue-spec
acceptance points: initial state reflects bound `value.multiValueStrategy`;
toggling on calls `onApply` with `multiValueStrategy: "explode"`; toggling
off restores `"combined"` and preserves the rest of the `PivotConfig`.
**Why:** The MultiValueStrategy toggle was already plumbed end-to-end by
earlier tasks — Rust `RecipeNodeBuilder::assignee_field_values` branches
on Combined/Explode (recipe_builder.rs:478–512) with an existing
`test_recipe_builder_multi_value_combined_vs_explode` snapshot; ViewConfigCache
already persists `pivot_config` (with `MultiValueStrategy::Explode` round-trip
pinned by `test_view_config_cache_round_trip_pivot_config`); the Svelte
checkbox + setToggle wiring landed in PR #71. Task 8 reduced to *adding the
component-wiring tests* called out in the issue spec. Reused the Task 3
extraction pattern (pure helper → sibling .ts module) per the repo
convention of "no Svelte/Tauri runtime in vitest".

**For Rusty's review focus:** the **Rust Explode branch was already
complete** — no changes to `recipe_builder.rs` in this PR. Same for
`ViewConfigCache` persistence. Review can focus on the test additions
in `RecipeBar.test.ts` and the tiny refactor in `RecipeBar.svelte` /
`recipeBarState.ts`.

**Validation (from `E:\prj\ghui-task8`):**
- `cargo fmt --all -- --check` → ✅ exit 0
- `cargo clippy --all -- -D warnings` → ⚠️ exit 101, 2 pre-existing
  `clippy::uninlined_format_args` errors in `ghui-app/src/telemetry.rs:188`
  and `ghui-app/src/updater.rs:90`; verified identical on `origin/main`
  (`00de52b`) by stashing this branch's changes and re-running. Not
  introduced by this PR. CI passes.
- `cargo test --all` → ✅ exit 0, 64 + 0 + 0 passed (regenerated ts-rs
  bindings byte-identical to HEAD; not staged)
- `cd app && npm run check` → ✅ exit 0, 0 errors, 0 warnings
- `cd app && npm test` → ✅ exit 0, **28 passed** (RecipeBar.test.ts:
  13 cases, +9 added by this commit)

**Commit:** `c819aed` (`pivoting/task8-explode-toggle`)
**PR:** https://github.com/damyanp/ghui/pull/75

**Hand-off note for Task 9 (parallel work on RecipeBar):** Diff is
minimal on purpose — only the Explode checkbox path was touched, and
the inline closures in `liveToggles` were replaced with calls to the
new `getToggleChecked` helper. Task 9 toggles ("show counts",
"collapse single-value", "hide closed", "show ghost ancestors"
behaviour, etc.) can add new entries to the `liveToggles` array and
new cases to `getToggleChecked` / `setToggle` without rebasing this
PR's structure.

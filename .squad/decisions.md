# Squad Decisions

## Phase 2 PR review batch — 2026-05-19

### 2026-05-18T17:19:25-07:00: PR #70 (Task 2 — RecipeNodeBuilder, DRAFT) — Rusty's verdict

**By:** Rusty (Lead)
**Verdict:** NEEDS_MORE_WORK
**CI:** UNKNOWN — only CodeQL (4/4 green). The `rust.yml` suite (`cargo fmt`, `cargo clippy`, `cargo test --all`, `npm run check`, `npm test`) does **not appear in the status checks at all**. The PR body contains an explicit firewall note: the cloud agent was blocked from compiling during its own run. Per the project's validation contract, all five commands must pass before marking ready-for-review. We cannot verify this.
**Additive-only:** YES — `NodeBuilder::add_nodes()` logic is not changed. `WorkItemTree.svelte` is not touched. The only NodeBuilder edits are two `is_ghost: false` struct-literal fields added to keep the code compiling after `is_ghost` was added to `Node`; this is the minimum necessary and does not alter behaviour.
**Spec fidelity:** Strong. What's done:
- `ghui-app/src/nodes/recipe_builder.rs` (new, 1144 lines) with `RecipeNodeBuilder::new()` and `build() → Vec<Node>`.
- All four axes implemented: `Pivot(field)` / `Group(field)` bucket-and-recurse, `Hierarchy` with ghost ancestor expansion, `Sort(field)` sort-and-pass-through.
- Ghost ancestor traversal from roots upward; `show_ghost_ancestors = false` skips the ghost path entirely.
- `MultiValueStrategy::Combined` (sorted join key) and `Explode` (one bucket per assignee) both implemented in `assignee_field_values`.
- `is_ghost: bool` added to `Node` struct and propagated to `Node.ts` binding and `TreeTable.svelte` type — Task 7 dependency satisfied.
- All four mandated tests present: `test_recipe_builder_preset_snapshots`, `test_recipe_builder_ghost_rows_for_mixed_epic_children`, `test_recipe_builder_multi_value_combined_vs_explode`, `test_recipe_builder_without_ghost_ancestors_flattens_buckets`.

What's missing / deviations:
1. **CI pass not confirmed.** The validation contract requires all five commands to pass before ready-for-review. They are unverified.
2. **PR title** is `"Add additive RecipeNodeBuilder …"`. Must be `pivoting(task2): …` per the coordination rules.
3. **Tests file location:** spec says `recipe_builder_tests.rs` (separate file). Tests are in the bottom of `recipe_builder.rs` as `#[cfg(test)] mod tests`. Not wrong Rust idiom, minor spec deviation, non-blocking.
4. **`parent_id()` does a linear scan** over all work items to find a parent when `get_parent()` returns `None`. Correctness is fine; performance on large datasets is not a Phase 2 concern but worth a comment in the code.

**Blocking items for ready-for-review:**
1. **Confirm CI.** Either: trigger a fresh commit so `rust.yml` runs and all five checks show green in the PR, OR post a comment quoting the pass/fail output for each of the five validation commands with evidence they were run on the branch head.
2. **Rename PR title** to `pivoting(task2): Add RecipeNodeBuilder for pivot recipes, ghost ancestry, and assignee bucketing` (or similar).

**Recommendation:** Original author iterates — both items are mechanical. Fix the title immediately. For CI, the author needs to either push a trivial commit to re-trigger `rust.yml`, or confirm locally and post the five-command output as a PR comment before flipping to ready-for-review.

---

### 2026-05-18T17:19:25-07:00: PR #71 (Task 3 — RecipeBar UI shell, DRAFT) — Rusty's verdict

**By:** Rusty (Lead)
**Verdict:** NEEDS_MORE_WORK
**CI:** ⚠️ Partial — only CodeQL checks appear (4/4 pass). The primary `rust.yml` pipeline is **absent**. DRAFT PRs may not trigger it on this repo. Cannot confirm `npm run check` or `npm test` pass from CI evidence alone. Author must post explicit pass/fail for all five commands before this goes ready-for-review.
**Additive-only:** ✅ Yes — `RecipeBar.svelte` is mounted only in `/dev/recipe-bar`. `WorkItemTree.svelte` is untouched. Zero deletions in the diff.

**Blocking items for ready-for-review:**
1. **PR title** — rename to `pivoting(task3): RecipeBar UI shell with presets and demo route`.
2. **Disabled stub toggles** — remove "Show counts", "Collapse single-valued groups", "Hide closed items" disabled checkboxes. They violate the project rule: *"Don't add UI elements until they have working functionality."* Leave them for Task 9.
3. **Parser coupling with PR #73** — either rebase onto #73 and replace `recipeText.parse/format` with async calls to `recipeParser`, or record an explicit decision. The current dual-parser situation is an architectural landmine for Task 6.
4. **CI confirmation** — author must post explicit pass/fail for all five validation commands. CodeQL passing is not sufficient.
5. **Minor** — replace `bg-error-50` / `text-error-700` with Skeleton dual-value tokens (`bg-error-50-950`, `text-error-700-300`).

**Recommendation:** Return to the PR author. Items 1 and 2 are mechanical. Items 3 and 4 require coordination: suggest landing PR #73 first, then rebasing #71 onto it and replacing the inline TS parser. Rusty will re-review after those four items are addressed.

---

### 2026-05-18T17:19:25-07:00: PR #72 (Task 4 — PivotConfig in AppState) — Rusty's verdict

**By:** Rusty (Lead)
**Verdict:** APPROVE_WITH_NITS
**CI:** PASS — all 7 checks green (format ✅, build ✅, frontend ✅, CodeQL ✅×3, CodeQL gate ✅). Completed 2026-05-16.
**Additive-only compliance:** YES — no changes to `NodeBuilder::add_nodes()` or `WorkItemTree.svelte` rendering logic.
**Spec fidelity:** Core deliverables land exactly as specced: `pivot_config: PivotConfig` in `AppState`, `PivotConfig::default()` on init, `get_pivot_config` and `set_pivot_config` Tauri commands.

**Issues found:**
- **`ghui-app/src/lib.rs` — Plan deviation (acceptable, plan should note):** The spec states "persist to the same per-project cache file that Filters already writes to". Filters were NOT previously persisted; the PR correctly invents a `ViewConfigCache` struct (filters + pivot_config) and the new file `~/view_config.ghui.json`. This is better than the spec described — the plan's premise was wrong. The plan should be amended to note that Task 4 also added filter persistence as a side effect. No code change needed.
- **`BufWriter` without explicit flush:** Uses `BufWriter::new(writer)` then passes it to `serde_json::to_writer_pretty`. This is the **existing pattern** used by `save_fields_to_appdata` and `save_workitems_to_appdata` on main — the PR did not introduce a new anti-pattern. Not a blocker for this PR, but the broader pattern is worth cleaning up separately.

**Plan amendments needed:**
1. Filters were not previously persisted — Task 4 adds filter persistence as a side effect via `ViewConfigCache`.
2. The cache file is `~/view_config.ghui.json` (fixed name, same flat-file convention), not a per-project-named file.

**Recommendation:** MERGE. Scribe should amend the plan for the two notes above. No code changes required.

---

### 2026-05-18T17:19:25-07:00: PR #73 (Task 5 — TS parser parity, deviated to Tauri delegation) — Rusty's verdict

**By:** Rusty (Lead)
**Verdict:** ESCALATE_TO_DAMYAN
**CI:** pass (all 7 checks green).
**Additive-only:** yes.

**Deviation evaluation:** The spec called for a pure-TypeScript implementation of `recipeParser.ts` that mirrors the Rust grammar, validated by loading the shared fixture file (`github-graphql/tests/fixtures/recipes.json`) and asserting identical parse trees. Instead, this PR makes `recipeParser.ts` a thin async wrapper that calls `invoke("parse_recipe")` and `invoke("recipe_to_string")` on the Rust side. The "parity" guarantee shifts from a fixture-backed test to "Rust is the only parser," which is logically stronger but has downstream consequences. The tests are hollow: they mock `invoke` and verify that the correct command name is called — they test zero actual parse logic.

**The critical cross-PR conflict:** PR #71 (RecipeBar, currently draft) already built a full synchronous TypeScript parser in `app/src/lib/recipeText.ts` with `parse(text) -> Axis[]` and `format(recipe) -> string`. PR #73's `recipeParser.ts` is async (`Promise<…>`) and lives alongside `recipeText.ts` — two competing TS parser APIs with incompatible signatures. Task 6 cannot wire them up without choosing one, and the choice has significant downstream consequence.

**Spec fidelity / amendment proposal:** If Damyan accepts the deviation, the plan amendment required is:
- Task 5 spec: Replace the "pure TypeScript implementation" paragraph with "thin async Tauri delegation wrapper in `recipeParser.ts`; tests verify IPC contract only, not parse semantics."
- Task 3 spec: Note that `recipeText.ts`'s `parse`/`format` functions must be removed or replaced with delegates to the async `recipeParser.ts`; `RecipeBar.svelte` must be made async-aware for recipe validation.

If Damyan rejects the deviation, the correct path is: (a) reject PR #73; (b) Task 5's deliverable is a new `recipeParser.ts` that wraps or re-exports `recipeText.ts`'s `parse`/`format` with the spec's `parseRecipe`/`recipeToString` API shape, plus a `recipeParser.test.ts` that imports the fixture file and asserts parse-tree parity.

**Recommendation:** Escalate to Damyan. This is an architectural fork with no locally-correct answer — both approaches have valid reasoning, and the cross-PR entanglement means the wrong pick will cost Task 6 significant rework.

---

### 2026-05-18T17:19:25-07:00: PR #70 (Task 2, DRAFT) — Livingston's test verdict

**By:** Livingston (Tester)
**Verdict:** READY_FROM_TEST_POV — *with a hard prerequisite: rust.yml must run and pass before promoting out of draft.*

**Test coverage of spec:**
- Preset snapshot tests (14/14 presets): ✅ Present — `test_recipe_builder_preset_snapshots` dynamically loads all presets from `recipes.json` and asserts full node-sequence output for each.
- Ghost rendering test: ✅ Present — `test_recipe_builder_ghost_rows_for_mixed_epic_children`.
- Combined vs Explode test: ✅ Present — `test_recipe_builder_multi_value_combined_vs_explode`.
- show_ghost_ancestors=false test: ✅ Present — `test_recipe_builder_without_ghost_ancestors_flattens_buckets`.

**All four spec-required tests are present and well-targeted.**

**Missing tests for ready-for-review:**
1. **rust.yml must run and pass.** Specifically the five-command validation suite: `cargo fmt --all -- --check`, `cargo clippy --all -- -D warnings`, `cargo test --verbose`, `npm run check`, `npm test`. Action: push a trivial commit (or un-draft + re-draft) to force rust.yml to trigger.

**Recommendation:** Author should push a commit to force rust.yml to run. If all 5 validation commands pass, this PR is clear to come out of draft from a testing perspective — no additional tests need to be written.

---

### 2026-05-18T17:19:25-07:00: PR #71 (Task 3, DRAFT) — Livingston's test verdict

**By:** Livingston (Tester)
**Verdict:** NEEDS_TESTS_FOR_READY

**Vitest coverage of spec:**
- Preset round-trip test: ✅ PRESENT — loops over all 14 `PRESETS` and asserts `format(parse(preset.recipe)) === preset.recipe`. However, does **not** exercise alias normalisation paths.
- Preset selection updates `value.recipe`: ✅ PRESENT — only one of 14 presets is spot-checked. Thin but present.
- `onApply` toggle emission test: ✅ PRESENT (at helper level).

**Vitest discipline:** ✅ CLEAN — no `@tauri-apps/api/core`, no Svelte runtime, no `invoke`.

**PR #73 coupling impact:** Two parsers coexist after both PRs merge with no cross-parity validation ensuring the two parsers agree.

**Missing tests for ready-for-review:**
1. **Parse error-case tests (2–3 minimum).** `parse()` throws on unknown fields, unknown axis kinds, missing parens, and `Hierarchy(Foo)`. None of these paths have a test.
2. **Alias normalisation round-trip.** `format(parse("Pivot(Issue_Type)"))` → `"Pivot(IssueType)"` (not `"Pivot(Issue_Type)"`).
3. **PR #73 parity test or explicit deferral note.** Either add a test that runs both parsers against the same inputs and asserts equal output, OR update the PR description to explicitly state that `recipeText.ts` is intentionally a permanent independent TS parser.
4. **CI validation.** Promote from DRAFT and let rust.yml run before merging.

**Recommendation:** Assign to **the Task 3 author** to add error-case and alias normalisation tests. Promote from DRAFT and confirm `npm test` passes in CI before requesting re-review.

**What changes my verdict to READY_FROM_TEST_POV:**
1. Error-case tests for `parse()` added (≥ 3 cases).
2. Alias normalisation test added (≥ 1 case).
3. PR #73 parity gap is either addressed by a test or explicitly called out in the PR description as a known deferral.
4. rust.yml CI passes (requires un-drafting).

---

### 2026-05-18T17:19:25-07:00: PR #72 (Task 4) — Livingston's test verdict

**By:** Livingston (Tester)
**Verdict:** APPROVE_WITH_TEST_ADDITIONS

**CI checks:** All seven checks green. Zero failing. This is the CI verdict.

**Coverage of new surface:**
- Round-trip cache test: ✅ PRESENT — `test_view_config_cache_round_trip_pivot_config`.
- Watcher DataUpdate test: ✅ PRESENT — `test_set_pivot_config_triggers_data_update`.
- ts-rs binding regenerated: YES — `app/src/lib/bindings/PivotConfig.ts` exists on the branch.
- Backward compat with serde defaults: ✅ COVERED — `test_view_config_cache_deserializes_legacy_filters_only`.

**Missing tests:**
1. **`test_set_filters_preserves_pivot_config_in_cache`** — HIGH PRIORITY. This PR modifies `set_filters` to also persist the full `ViewConfigCache` (filters + pivot_config together). There is no test verifying this interaction. The risk: calling `set_filters` would silently overwrite the saved `pivot_config` with `Default` if `view_config_cache()` were wrong.

2. **`test_get_pivot_config_returns_set_value`** — LOW PRIORITY. A direct `get → set → get` round-trip test. Worth adding but not a blocker.

**Recommendation:** Add `test_set_filters_preserves_pivot_config_in_cache` (item 1 above) then merge. The CI is green, the three required tests from the spec are present, backward compatibility is verified.

**What would change my verdict to APPROVE outright:** Item 1 added. That single test covers the only genuine regression risk I see in the new `set_filters` side-effect.

---

### 2026-05-18T17:19:25-07:00: PR #73 (Task 5 — Tauri delegation) — Livingston's test verdict

**By:** Livingston (Tester)
**Verdict:** REJECT_LOST_TEST_SURFACE

**Test approach analysis:**
`recipeParser.test.ts` (56 lines, 3 tests) mocks `@tauri-apps/api/core`'s `invoke` and verifies command wiring only. These are shim-wiring tests — they tell you nothing about what the Rust parser actually produces for any input.

**What stopped being tested:**
- **No real parse inputs are exercised at the TS level.** Every "parse" in the TS test is a mock return value.
- **`recipes.json` is completely unused by any TS test.** The spec required loading this fixture and asserting the same parse tree for every entry. That contract is gone.
- **The Tauri commands themselves have no Rust unit tests.** The command surface itself (error serialisation, `TauriCommandResult` wrapping, `Vec<Axis>` round-trip through serde) is untested.
- **`recipeToString` has no error handling in the shim.** No test covers the `recipeToString`-fails path.

**Parity story:** PR #71 landed a fully-functional synchronous TypeScript parser in `recipeText.ts`. `RecipeBar.svelte` calls this TS `parse()` synchronously. After both PRs are merged, the frontend has **two coexisting parsers that are never verified to agree**.

**Missing tests:**
1. **Fixture-driven integration test for the shim.** Load `recipes.json` (or a subset) in `recipeParser.test.ts`, feed each key string to `parseRecipe`, and assert the returned `recipe` array matches the fixture value.
2. **`recipeToString` error path.** Add a test: `invokeMock.mockRejectedValue("...")` for `recipeToString`, assert the error propagates.
3. **Rust command surface smoke test.** A `#[cfg(test)]` test in `data.rs` that calls `parse_recipe` and `recipe_to_string` as plain functions with each fixture entry and asserts round-trip.
4. **Cross-parser agreement test.** Either delete `recipeText.ts::parse()` and update `RecipeBar.svelte` to use `parseRecipe`, or add a test that runs both parsers against the same inputs and asserts identical output.

**Recommendation:** **Do not merge as-is.** The immediate ask is small: add test 2 (error path for `recipeToString`) and test 3 (fixture-driven shim test using real-shaped data). The larger issue — two coexisting parsers with no agreement test — should be addressed before Task 6 lands.

**What would change my verdict to APPROVE:**
1. `recipeParser.test.ts` exercises at least one fixture entry with a real-shaped `Axis[]` payload (not a hand-rolled constant).
2. `recipeToString` error propagation is explicitly tested.
3. Either `recipeText.ts::parse()` is removed/deprecated with a note, or a cross-parser agreement test is added covering `recipes.json`.

---

## Wave A — Phase 2 PR fixes (Wave A) — 2026-05-19

### 2026-05-18T17:19:25-07:00: PR #72 — set_filters preservation test added
**By:** Basher
**What:** Added `test_set_filters_preserves_pivot_config_in_cache` per Livingston's verdict.
**Why:** Closes the test gap. PR #72 ready to merge once CI confirms green.

### 2026-05-18T17:19:25-07:00: PR #73 — fixture + error tests added
**By:** Livingston
**What:** Added fixture-driven shim verification and recipeToString error-path test.
**Why:** Per Damyan's call to keep Tauri delegation, parity is now enforced by the shim test forwarding all 14 fixtures + the Rust parser's existing fixture tests. PR #73 ready to merge once CI confirms green.

---

## Governance

- All meaningful changes require team consensus
- Document architectural decisions here
- Keep history focused on work, decisions focused on direction

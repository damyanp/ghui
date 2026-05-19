# Project Context

- **Owner:** Damyan Pepper
- **Project:** ghui — Tauri 2 + SvelteKit GitHub project management desktop app
- **Stack:** Rust (Tauri 2, anyhow, ts-rs, insta), TypeScript (Svelte 5 runes, Tailwind 4 / Skeleton, vitest), GitHub GraphQL
- **Current focus:** Pivoting plan PR review for tests. All four Phase 2 PRs (#70, #71, #72, #73) need test review — coverage, fixture parity, validation-suite pass. Cross-language parity via `github-graphql/tests/fixtures/recipes.json` is the critical contract.
- **Created:** 2026-05-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->

- 📌 Test naming: `test_<action>_<scenario>`.
- 📌 `TestData` builder: `data.build().status("Active").assignees(vec!["user"]).add()`. `WorkItemId` is incrementing numeric string starting "1" per TestData instance (`next_id` resets on `Default`).
- 📌 Snapshot testing with `insta` is available in `github-graphql`. NodeBuilder tests live in `ghui-app/src/nodes.rs` (run with `cargo test -p ghui-app`; requires `libdbus-1-dev` on Linux).
- 📌 Vitest tests: `app/src/lib/*.test.ts`. Plain TS, no Svelte/Tauri imports. `invoke` is not available — extract pure helpers from `.svelte.ts` classes and test those.
- 📌 Validation suite (all five together, every PR): `cargo fmt --all -- --check` → `cargo clippy --all -- -D warnings` → `cargo test --all` (or `-p github-graphql`) → `cd app && npm run check` → `cd app && npm test`.
- 📌 PRs #42 and #44 caused `Cargo.lock` desync. Investigate, never blindly revert. Confirm workspace `Cargo.toml` package versions match lockfile `[[package]]` entries before deciding the diff is "unrelated".
- 📌 Pivoting Task 1 (foundation, landed): parser in `github-graphql/src/pivot.rs`. Round-trip every preset from the prototype. Error cases: unknown field, unknown axis, missing parens. Snapshot the fixture.
- 📌 Task 5 deviated from a hand-rolled TS parser to delegating through Tauri (per PR #73 title) — verify cross-language parity is still tested somehow.
- 📌 Team update (2026-05-19): Phase 2 review batch complete. Parity-contract lesson: when parallel Task N and Task N+k deviations appear (async parser in #73 vs sync parser in #71), the fixture contract breaks if both land. Tests must explicitly validate the pair or refuse to merge one. Cross-parser parity is not implicit. On fixture-driven specs, add a "cross-impl agreement" test requirement. Rejection verdicts for #73: lost fixture coverage, hollow mocked tests, cross-PR collision with #71.
- 📌 Fixture-driven shim test pattern (PR #73, 2026-05-18): When a TS module delegates to a Tauri command, verify the shim by loading `github-graphql/tests/fixtures/*.json` with `fs.readFileSync` (plain Node, no build step), iterating over every entry, mocking `invoke` with `mockResolvedValueOnce`, calling the shim function, and asserting both the `invoke` call args and the returned value. Fixture path from `app/src/lib/` is `../../../github-graphql/tests/fixtures/` (3 levels up reaches the repo root). Use `import.meta.dirname` for reliable resolution in vitest. This replaces duplicating parse logic in TS tests — the Rust parser's own fixture tests handle parse correctness; the TS shim test only checks IPC wiring.
- 📌 vitest passes ≠ svelte-check passes; always run BOTH `npm test` AND `npm run check`. Node API imports break svelte-check unless `types: ['node']` is in tsconfig — prefer Vite JSON imports for fixtures.
- 📌 **Structural invariants over `Vec<Node>` (2026-05-26, PR #79):** Opaque string-snapshot tests (`assert_eq!(actual, "<giant literal>")`) are insufficient for tree-output assertions — they reward "render what you currently render" but cannot catch invariant violations the eye misses. Concrete example: `test_recipe_builder_preset_snapshots` had `item 4` recorded twice in `Pivot(Epic) → Hierarchy` (once as ghost in EpicB, once as real in `epic=(none)`) with the same `Node.id`, and the snapshot literal "passed" green for months while crashing Svelte's keyed `{#each}` blocks at runtime with `each_key_duplicate`. Pattern: split render helpers into `build_<thing>_nodes(...) -> Vec<Node>` + `format_nodes_string(&[Node]) -> String`, then run structural assertions on the `Vec<Node>` (uniqueness via `HashSet`, level monotonicity, id-presence in source map) BEFORE the snapshot compare. Add a per-snapshot `total=N unique_ids=M` header line so future duplicates jump out in diffs.
- 📌 **Matrix tests catch what snapshot tests can't (PR #79):** the existing snapshot ran only with `Combined + ghost=true`. Building a matrix test over `every preset × show_ghost_ancestors∈{true,false} × multi_value_strategy∈{Combined,Explode}` surfaced a previously-invisible duplicate: `Pivot(Assignee) → Group(Epic)` with `ghost=false strategy=Explode` produces duplicate id "4" when an item is assigned to multiple users (item is rendered once in each assignee bucket). Lesson: when a builder takes orthogonal flags, snapshot-test ONE combination but matrix-test ALL combinations against structural invariants.
- 📌 **Failure message convention for matrix tests:** every assertion in a matrix should include a `label` string like `format!("preset={preset_name:?} ghost={show_ghost_ancestors} strategy={strategy:?}")` so a CI failure points straight at the broken combination. Without this, "duplicate ids in some test" is unactionable.


### 2026-11-20T14:50:00Z: PR #79 rebased on main after #78 merged — all green
**By:** damyanp (via Copilot, dispatched by coordinator)
**What I did:**
- Rebased `test/node-id-invariants` onto `origin/main` (which now contains #78, Linus's path-prefixed Node.id fix).
- Resolved one conflict in `ghui-app/src/nodes/recipe_builder.rs`: kept both #78's regression test (`test_recipe_node_builder_no_duplicate_node_ids_with_ghost_ancestors_across_buckets`) and my matrix invariant test (`test_recipe_builder_node_id_uniqueness_invariant`) — they're complementary, not duplicative.
- Fixed compile error in my `assert_work_item_ids_present` helper: `NodeData::WorkItem` became a struct variant with a `work_item_id` field in #78. Updated to `if let NodeData::WorkItem { work_item_id } = &n.data` — anticipated by the helper's own doc comment.
- Force-pushed with `--force-with-lease`.
- Updated PR #79 body via REST API (`gh api -X PATCH /repos/.../pulls/79`) — `gh pr edit` failed with exit 1 due to a Projects-classic GraphQL deprecation warning, but the REST endpoint worked cleanly.

**Bonus case verdict (Pivot(Assignee) → Group(Epic), Explode, ghost=false, duplicate id "4" for multi-assigned items):**
- Coordinator's prediction was correct. The fix in #78 uses `child_path` when constructing work-item node ids, and `child_path` includes the assignee bucket. So distinct assignees yield distinct path-prefixed ids (e.g., `path/assignee=alice/epic=id(EpicA)/4` vs `path/assignee=alice+bob/epic=(none)/4`).
- The matrix test iterates `preset × ghost × Combined/Explode`, so the previously-failing Pivot(Assignee)+Explode case is now exercised on every run and passes. **No follow-up bug.**

**Validation results (rebased branch):**
| # | Command | Result |
|---|---------|--------|
| 1 | `cargo fmt --all -- --check` | ✅ Pass |
| 2 | `cargo clippy --all -- -D warnings` | ⚠️ 2 pre-existing errors in `telemetry.rs` / `updater.rs` (commits `f6a8e7c`, `b05cf7b` on main, pre-date both #78 and my branch) — out of scope for this PR |
| 3 | `cargo test -p ghui-app` | ✅ Pass (50/50) |
| 4 | `cd app && npm run check` | ✅ Pass (0 errors, 0 warnings) |
| 5 | `cd app && npm test` | ✅ Pass (54/54) |

**Lesson — anticipatory helper docs paid off:** When I originally wrote `assert_work_item_ids_present`, I added a doc comment: *"Assumption: on current main, node.id IS the WorkItemId. If a fix changes the id format to a path-prefixed string, this helper will need to extract the work item id (likely the trailing segment) instead."* That exact scenario happened. The note pointed straight at the fix and made the post-rebase update trivial. Recommend the pattern: when a test helper depends on a structural assumption that a future fix might change, name the assumption explicitly in a doc comment.

**Lesson — `gh pr edit` vs REST for repos with Projects-classic:** Repos that have Projects-classic linkage (this one does) cause `gh pr edit` to exit 1 with a GraphQL deprecation warning *even when the edit would succeed*. Workaround: `gh api -X PATCH /repos/{owner}/{repo}/pulls/{n} --input <json>` — bypasses the GraphQL path entirely and exits 0 cleanly.

**Lesson — clippy debt on main is real:** `cargo clippy --all -- -D warnings` currently fails on main due to `uninlined_format_args` in `telemetry.rs:188` and `updater.rs:90`. Not my fix to make in this PR, but flagging — CI for any new PR will hit this unless a chore PR cleans it up or those lints are downgraded.

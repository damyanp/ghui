# Project Context

- **Owner:** Damyan Pepper
- **Project:** ghui — Tauri 2 + SvelteKit GitHub project management desktop app
- **Stack:** Rust (Tauri 2, anyhow, ts-rs, insta), TypeScript (Svelte 5 runes, Tailwind 4 / Skeleton, vitest), GitHub GraphQL
- **Current focus:** Coordinating the pivoting plan (`docs/pivoting-implementation-plan.md`) — replacing the hardcoded `Pivot(Epic) → Hierarchy` grouping in `NodeBuilder` with a configurable `Vec<Axis>` recipe driven from a toolbar. Phase 1 (Task 1: parser foundation) has landed. Phase 2 (Tasks 2–5) has 4 open PRs (#70, #71 draft; #72, #73 ready). Phase 3 (Task 6 wire-up) is blocked on Phase 2 landing.
- **Created:** 2026-05-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->

- 📌 Project setup (2026-05-19): The pivoting plan is divided into 4 phases. Phase 2 tasks are **additive only** — they must not modify `NodeBuilder::add_nodes()` or `WorkItemTree.svelte` rendering logic. Those lines are reserved for Task 6. Enforce this rule during PR review.
- 📌 Validation contract: All five commands must pass before merge — `cargo fmt --all -- --check`, `cargo clippy --all -- -D warnings`, `cargo test --all`, `cd app && npm run check`, `cd app && npm test`. Quote pass/fail in every review.
- 📌 PR title convention: `pivoting(taskN): …` so the open PR list groups by task.
- 📌 Recipe fixture file `github-graphql/tests/fixtures/recipes.json` is the contract between Tasks 2 and 5 — the Rust and TS parsers must produce identical parse trees against it.
- 📌 Async API mismatch risk (2026-05-18, PR #73 review): When a parallel-task agent deviates to a Tauri-delegation model, the async API it produces (`Promise<…>`) will conflict with any synchronous API another parallel agent already built for the same logical function. PR #71 built a sync TS parser (`recipeText.ts`) that `RecipeBar.svelte` consumed; PR #73's async `recipeParser.ts` is incompatible without a rewrite. Catch this in review before it reaches Task 6.
- 📌 "Tested pair" vs "single source of truth" tradeoff (2026-05-18, PR #73 review): Delegating from TS to Rust via Tauri eliminates parser divergence but produces hollow tests (mocked `invoke` verifies command name, not behavior). The fixture-parity model is stronger for test coverage but requires two implementations. When agents make this tradeoff autonomously, escalate to the human — it's an architectural fork, not a local decision.
- 📌 Parallel agents can collide on the same logical file under different names (2026-05-18): Task 3 (PR #71) implemented the TS recipe parser as `recipeText.ts`; Task 5 (PR #73) produced a competing `recipeParser.ts`. Both satisfy the same logical requirement but with incompatible APIs. The fixture-contract coordination rule (rule 4) was intended to prevent this but didn't name the file explicitly enough.
- 📌 Plan spec premises can be wrong (2026-05-18, PR #72 Task 4): The plan said "persist to the same per-project cache file that Filters already writes to" — but Filters had no persistence path before Task 4. When the spec assumes an existing behavior that doesn't exist, the implementing agent correctly invents the mechanism (`ViewConfigCache` / `view_config.ghui.json`). Review the implementation's reasoning, not just the diff against the letter of the spec. File this as a plan amendment rather than a rejection.
- 📌 `get_appdata_path` uses a flat convention (2026-05-18, PR #72 Task 4): All appdata files follow `~/{name}.ghui.json` with a fixed name, NOT a per-project name. `fields.ghui.json`, `work_items.ghui.json`, `view_config.ghui.json` — all global, single per machine. Plan docs that say "per-project cache file" are misleading; the actual pattern is one file per data type, shared across projects.
- 📌 `BufWriter` without explicit flush is an existing pattern (2026-05-18, PR #72 Task 4): `save_fields_to_appdata` and `save_workitems_to_appdata` on main both use `BufWriter::new(writer)` passed to `serde_json::to_writer_pretty`. This predates PR #72. Drop-based flush silently swallows errors. Worth a cleanup PR but don't block individual feature PRs that follow the established pattern.
- 📌 Team update (2026-05-19): Phase 2 review batch complete. PR #72 ready to merge with 1 test. PR #73 needs architectural decision from Damyan (async vs sync parser tradeoff). PR #70 and #71 need CI runs and title fixes. Lessons learned: (1) PR title convention `pivoting(taskN):` must be enforced early. (2) Dead-stub UI toggles violate "no UI before functionality" — detect and reject during review. (3) Parallel agents can collide on parser implementation strategy (sync TS vs async Tauri delegation) — catch cross-PR coupling in architectural review and escalate if unresolved.
- 📌 Task 6 wire-up (2026-05-19): Phase 2 tasks pre-completed part of Task 6's spec — `get_pivot_config`/`set_pivot_config` were registered in `tauri::generate_handler!` by PR #72, not Task 6. When a later phase's sub-tasks arrive pre-done by an earlier phase, flag as a plan deviation rather than silently re-doing or skipping them. The single Rust call site switch (`NodeBuilder` → `RecipeNodeBuilder`) is a one-liner; the real task risk is default-recipe equivalence verification and `NodeBuilder` deletion safety.

## 2026-05-19 — PR #78 review: duplicate Node.id with cross-bucket ghost ancestors

**Branch:** `fix/duplicate-node-keys` @ `8316c84` (author: Linus, requested by Damyan)
**Decision:** APPROVE (posted as `--comment` — GitHub blocks self-approval since Damyan authored the PR)

### What the fix does
- `Node.id` for work-item nodes becomes `child_path(path, id)` (path-prefixed), matching the convention groups already used. Render-position-unique.
- `NodeData::WorkItem` gains a `work_item_id: WorkItemId` field so the frontend can recover the semantic id without parsing the path string.
- 8 frontend call sites switched from `row.id`/`node.id` (treated as WorkItemId) to `node.data.workItemId` after narrowing on `node.data.type === "workItem"`. Render-position uses (`{#each}` key, `data-row-id`, expand state, visibility set, drag-tracking) intentionally stay on the render-id.
- `findPrimaryRow` semantics changed: now matches on `data.workItemId` (skips groups), still returns the row whose `.id` is the render-key (the scroll target).

### What I verified
1. **Path uniqueness across recipes.** Traced Pivot, Group, Hierarchy, Sort, Combined, Explode, multi-level. Buckets get distinct paths via `group_node_id(path, field, key)`. Hierarchy descent appends `/<parent_id>` per level. Same-id-multiple-bucket (the bug case) gets distinct prefixes via per-bucket `group_id`. ✓
2. **TS narrowing.** All 8 call sites narrow before access. The `handleDrop` else-branch correctly became `else if (targetNode.data.type === "workItem")` — exhaustive on the two-variant union, strictly safer than implicit fallthrough. ✓
3. **`findPrimaryRow` scroll contract.** Still returns render-id; `jumpToRowById` and `[data-row-id="..."]` still find the right DOM row. New test asserts `jumpTo` receives the render-id, not the workItemId. ✓
4. **Snapshot stability.** `render_recipe_nodes` formats work-item lines by `work_item_id.0`, not `node.id`. Group lines use `node.id` but groups were already path-prefixed pre-PR. So `test_recipe_builder_preset_snapshots` literal correctly didn't need updating. ✓
5. **Regression test quality.** Reproduces exact failure mode (`Pivot(Epic) + Hierarchy`, ghost ancestors, parent in EpicA, child in EpicB). Asserts uniqueness AND dual real/ghost occurrence. Strong. ✓
6. **Anti-patterns.** No new `.unwrap()`/`.expect()` in non-test code. No path-string parsing on the frontend (the only `.split()`/`.substring()` hits are pre-existing code on `resourcePath` and date strings, unrelated to Node.id). ✓
7. **Pre-existing clippy errors.** `git diff origin/main -- ghui-app/src/telemetry.rs ghui-app/src/updater.rs` is empty. Not this PR's problem. ✓

### Latent drag bug — Linus's "only worked by coincidence" claim
True in the strict sense: old code set `Change.workItemId = draggedRowId` and `setParent.value = droppedOntoRowId`, relying on `Node.id == WorkItemId.0`. User-visible? No — the identity held, so values were semantically correct. The fix is necessary now (since Node.id is no longer the WorkItemId), but not a separately ship-worthy bug.

### Nits
- `app/src/lib/ghostRouting.ts:16` comment says "PR #79" — should be #78. Mentioned in PR review as non-blocking.

### Patterns learned
- **Render-id vs semantic-id separation.** Once you key a `{#each}` block on something, that thing is a render-position key, not a semantic id. The instant you have a recipe where the same semantic id can legitimately appear twice in the rendered list, you MUST split them. Group nodes already had this pattern via `group_node_id` — work items just needed parity. Worth promoting to a project-level skill: "If two rows can render with the same semantic id, key on a path-prefixed render-id and carry the semantic id as a data payload."
- **Self-approval is blocked by GitHub.** For solo-author repos, post reviews as `--comment` and state the verdict explicitly in the body. Decision is recorded in `.squad/decisions.md` for team-side authority.

- 📌 PR #79 review (2026-05-19): Livingston's testing-gap follow-up to #78 — adds structural Node.id uniqueness, level monotonicity, and WorkItem id presence invariants in a matrix test (14 presets × 2 ghost × 2 strategy = 56 combinations). Also augments the snapshot with `total=N unique_ids=N` headers so duplicate re-recording becomes visually obvious. Verified Explode-bonus case (item 4 alice+bob multi-assigned) mentally — distinct `path/assignee=…` prefixes via #78's `child_path` fix resolve it. Posted COMMENT review with implied APPROVE vote (Damyan authored, can't formally approve). One stale doc-comment nit on `assert_work_item_ids_present` (claims node.id == WorkItemId — no longer true post-#78). Recommended merge.
- 📌 Test-gap pattern (general): when a snapshot test exercised the buggy code path but didn't catch the bug, the snapshot itself locked the wrong output in as expected. The fix is structural invariants layered on top of the snapshot, plus a visible aggregate (like `total=N unique_ids=N`) in the snapshot text so a future re-record makes the regression visible. PR #79 is the canonical example.
- 📌 Bug-class taxonomy worth keeping ahead of: Svelte's keyed `{#each}` panics at runtime on `each_key_duplicate` — duplicate Node.id values are silent in unit tests but fatal in the UI. Always assert Node.id uniqueness for any code path that emits keyed lists.

### 2026-05-19: PR #79 review — structural Node.id uniqueness invariants
**By:** Rusty (Lead) — review requested by Damyan
**What:** Reviewed PR #79 (`test/node-id-invariants`, commit `09144c6`), Livingston's follow-up to #78. Posted COMMENT review on GitHub with implied vote = **APPROVE** (formal approval blocked: Damyan authored). Recommended merge.

**Scope of the PR:**
1. Augmented `test_recipe_builder_preset_snapshots` with an in-test uniqueness assertion (hard stop before the opaque string compare) plus a per-preset `total=N unique_ids=M` header line in the snapshot literal.
2. New `test_recipe_builder_node_id_uniqueness_invariant` — matrix test over every preset in `recipes.json` × `show_ghost_ancestors ∈ {true,false}` × `multi_value_strategy ∈ {Combined,Explode}` (14 × 2 × 2 = 56 combinations) asserting (a) Node.id uniqueness, (b) level monotonicity (first node level 0; never jump down >1), (c) WorkItem id presence in `work_items`.

**Why approve:**
- The three invariants are the right minimum set: each catches an independent silent-bug class, and they're cheap to evaluate.
- Matrix coverage is complete: enumerates all preset keys (sorted) × both ghost values × both strategies.
- Explode bonus case verified mentally: item 4 (`parent_none`, assigned alice+bob) under `Pivot(Assignee) → Group(Epic)` + Explode produces `path/assignee=alice/epic=(none)/4` and `path/assignee=bob/epic=(none)/4` — distinct ids, resolved by #78's `child_path` fix.
- Failure messages include preset/ghost/strategy label + specific violation detail (colliding ids, level jump prev→curr, missing work_item_id) — actionable at CI.
- Snapshot header (`total=N unique_ids=N`) is the right place for the deduped count: any future regression makes itself visible in the diff.

**Nits (non-blocking):**
- Doc comment on `assert_work_item_ids_present` says *"Assumption: on current main, `node.id` IS the `WorkItemId`. If a fix changes the id format to a path-prefixed string, this helper will need to extract the work item id..."* That assumption is stale — #78 has landed and `node.id` IS path-prefixed. The helper is correctly checking `work_item_id` from the `NodeData::WorkItem` variant, not `node.id`, so the code is right; the comment just describes a world that no longer exists. Trim it.

**Follow-up issues to file (out of scope for #79):**
- `has_children` consistency invariant: if `true`, next node should be at `level + 1`; if `false`, next node should be at `level <= current`.
- Empty-group detection: a `NodeData::Group` followed immediately by a node at `level <= group.level` is a builder bug.

**Explicitly rejected as not-an-invariant:** "ghost nodes always have a real counterpart for the same WorkItemId somewhere." It happens to hold in current `Pivot(Epic) → Hierarchy` Combined output, but the semantics permit ghosts to exist purely for filter-context (a filter-matched descendant pulls in a ghost ancestor that itself matches no bucket). Risk of false positives outweighs the value.

**CI:** format ✅, frontend ✅, CodeQL (actions/javascript-typescript/rust) all ✅; build still IN_PROGRESS at time of review — Livingston validated locally per the request and didn't ask for re-run.

**Process note (general):** When a snapshot test exercises a buggy code path but doesn't catch it, the snapshot itself becomes complicit by locking the wrong output in as "expected." Future test-gap follow-ups should layer structural invariants on top of the snapshot (not replace it) plus a visible aggregate inside the snapshot text so re-recorded regressions become visually obvious. PR #79 is the canonical example of this pattern in this repo.

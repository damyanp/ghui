### 2026-05-19: PR #78 (`fix/duplicate-node-keys` @ `8316c84`) — APPROVE
**By:** Rusty (Lead, code review)
**Requested by:** Damyan
**Verdict:** APPROVE — recommended merge as-is.

**Posted as `gh pr review --comment`** because GitHub blocks self-approval (Damyan authored the PR). The review body explicitly states "APPROVE … Ship it." Decision is authoritative here regardless of the GitHub review status flag.

**What was fixed.** Svelte `each_key_duplicate` crash in `Pivot(Epic) + Hierarchy + show_ghost_ancestors` recipes. Root cause: `Node.id` was the bare `WorkItemId`, but the same work item legitimately appears in multiple Pivot buckets (real in its own bucket, ghost ancestor in others). Two Node entries with the same id → keyed `{#each}` rejects → expand/drag/visibility (all keyed on `row.id`) silently fail for downstream rows.

**Architectural takeaway (decision-level, applies beyond this PR).**

> **Render-id vs semantic-id MUST be separate fields whenever the same semantic id can legitimately appear in multiple rendered rows.** `Node.id` is a render-position-unique key (now `child_path(path, id)`, matching the existing `group_node_id` convention). The semantic identifier (`WorkItemId`) lives in the `NodeData::WorkItem { work_item_id }` payload. Frontend code uses `row.id` / `node.id` for render-position concerns (`{#each}` key, `data-row-id`, expand/visibility/drag state, scroll target). It uses `node.data.workItemId` (after narrowing on `node.data.type === "workItem"`) for semantic lookups (`workItems[...]`, `Change` records, `itemUpdateBatcher`). **Path-string parsing of `Node.id` on the frontend is forbidden** — that would defeat the whole separation. If a new variant of `NodeData` is added later, every site that does `data.type === "workItem"` narrowing must be re-audited.

**Why approve.**
- Dedupe is correct across all recipe shapes (Pivot, Group, Hierarchy, Sort, Combined, Explode, multi-level). Bucket paths get unique prefixes via `group_node_id`; hierarchy descent appends `/<parent_id>` per level.
- All 8 frontend call sites narrow on `node.data.type === "workItem"` before accessing `workItemId`. Drag handler's `else` correctly became `else if (targetNode.data.type === "workItem")` (exhaustive on the two-variant union, strictly safer).
- `findPrimaryRow` refactor preserves the scroll-target contract: matches on `data.workItemId`, returns the row whose `.id` is the render-key for `[data-row-id="..."]` lookup.
- Regression test (`test_recipe_node_builder_no_duplicate_node_ids_with_ghost_ancestors_across_buckets`) reproduces exact failure mode and asserts both uniqueness AND dual real/ghost occurrence.
- Snapshot test (`test_recipe_builder_preset_snapshots`) correctly stays stable because `render_recipe_nodes` formats work-item lines by `work_item_id.0`, not `node.id`.
- No new `.unwrap()`/`.expect()` in non-test code. No frontend path-string parsing.
- `telemetry.rs` and `updater.rs` untouched (pre-existing clippy errors are not this PR's problem — verified via `git diff origin/main`).

**Single nit (non-blocking, doc-only):** `app/src/lib/ghostRouting.ts:16` doc-comment references "PR #79" — should be "#78". Surfaced in PR review.

**Recommendation:** merge as-is. The PR #79 typo can be picked up on the next pass, no need to block a hot-fix on a doc reference.

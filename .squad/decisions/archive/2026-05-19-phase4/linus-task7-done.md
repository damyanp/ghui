### 2026-05-19: Task 7 (Ghost-row visuals + click routing) — PR #76

**By:** Linus (Frontend Dev)
**Requested by:** Damyan Pepper (via Copilot CLI coordinator)

## Summary

Implemented Phase 4 / Task 7 of the pivoting plan. PR #76 is open from `pivoting/task7-ghost-visuals` (HEAD `ae0030e`) into `main`, rebased on top of `origin/main` at `9a7dc1f`.

- **PR:** https://github.com/damyanp/ghui/pull/76
- **Branch:** `pivoting/task7-ghost-visuals`
- **Commit SHA:** `ae0030ea809ea8e8ae0833d1fa97b9f3f909dd6e`
- **Issue closed:** #66

## Files changed (4 files, +241 / -2)

| File | Change |
|---|---|
| `app/src/components/TreeTable.svelte` | +49 / -2 — ghost styling in `getRowClass`; `draggable={isRowDraggable(row)}` instead of `{!row.isGroup}`; `dragStartHandler` early-returns on ghost; new `handleRowClick` attached as `onclick` that routes ghost clicks to the primary via `scrollIntoView`. |
| `app/src/components/WorkItemTree.svelte` | +27 — new `jumpToRowById(id)` DOM helper; `getContextMenuItems` short-circuits when `node.isGhost` and returns `ghostContextMenuItems(rows, node.id, jumpToRowById)`. |
| `app/src/lib/ghostRouting.ts` | **new** — dependency-free helpers: `findPrimaryRow`, `isRowDraggable`, `ghostContextMenuItems`. |
| `app/src/lib/ghostRouting.test.ts` | **new** — 10 vitest cases covering every helper, including the "ghost with no primary in view" fallback. |

## Design decisions worth flagging

1. **The plan was slightly imprecise about the file split.** It said "modify `WorkItemTree.svelte` — apply muted CSS class". The actual row rendering lives in the generic `TreeTable.svelte`. I put the ghost styling, drag suppression, and click handler in `TreeTable.svelte` (so the behaviour follows the row, regardless of which feature mounts the table) and put the ghost-aware *context menu* in `WorkItemTree.svelte` (because that's where `getContextMenuItems(node, column): MenuItem[]` is defined). Both files end up touched, which is what the plan said.

2. **Helper module exports `findPrimaryRow`, not `resolveGhostClick`.** Earlier draft had `resolveGhostClick(rows, clickedRowId)` that looked up the clicked row by id, then walked to the primary. The first test exposed why that's broken: **ghost and primary share an id** (both are the WorkItemId), so `rows.find(r => r.id === clickedId)` always returned the primary, never the ghost, and the early-return masked the rest of the logic. The TreeTable callers already have the actual `MRow<T>` object in scope (with `isGhost` on it), so the indirection was useless. Removed it; callers gate on `row.isGhost` themselves and call `findPrimaryRow` directly. The remaining helpers are unambiguous.

3. **Ghost styling order in `getRowClass`.** Placed `if (row.isGhost) return ...` **before** the `isModified` / `modifiedDescendent` branches so a row that is somehow both a ghost AND modified still looks ghost-y. Ghosts are reflections — they shouldn't claim the "this row has edits" affordance even by accident.

4. **No `hover:bg-*` on ghosts.** The base row class is `cursor-default` — kept as-is for ghosts so they don't *advertise* clickability. The ghost is muted (`italic text-surface-500-500`) with no hover background, but clicking still works (and the context menu has the explicit "Jump to primary occurrence" affordance). This matches the spec's "suppress hover affordances".

5. **`CSS.escape` on the primary id in the selector.** WorkItemIds are GraphQL global ids, generally safe, but cheap insurance against any future id format that contains characters with CSS-selector meaning.

6. **Drag suppression is belt-and-braces.** Both `draggable={isRowDraggable(row)}` (false for ghost) AND `dragStartHandler` early-returns + `e.preventDefault()` on ghost. The latter guards against a stale `draggable` attribute (e.g. a row that flipped from non-ghost to ghost while a drag was being prepared by the browser).

## Out of scope — recommended follow-ups for Rusty

- **Auto-expand collapsed ancestors when jumping to a primary.** Currently if the primary is hidden because an ancestor isn't expanded, `scrollIntoView` is a no-op + debug log. The plan acceptance ("scroll to + select it") didn't mention expansion so I deferred. Worth a separate issue.
- **Visual flash on the jumped-to row.** A brief highlight (~1.5s) would be a nice cue after the smooth scroll. Held off because it would require introducing per-row selection state to TreeTable, which is a larger design call than this task warranted.

## Validation results (from `E:\prj\ghui-task7`)

| Command | Exit | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | **0** | clean |
| `cargo clippy --all -- -D warnings` | **101** | **2 pre-existing errors only** — `clippy::uninlined_format_args` on `ghui-app/src/telemetry.rs:188` and `ghui-app/src/updater.rs:90`. Neither file touched by this PR. CI uses an older Rust toolchain that doesn't lint this and passes. Flagged in PR body per the task spec. |
| `cargo test --all` | **0** | 64 tests pass in `ghui_lib`, all suites green |
| `cd app && npm run check` | **0** | 0 errors, 0 warnings |
| `cd app && npm test` | **0** | 32 tests pass (10 new in `ghostRouting.test.ts`) |

## Reviewer notes

- The PR body has the full validation table, the design notes, and the out-of-scope list — no need to re-read this file.
- ts-rs regenerated all 39 binding files on `cargo test --all` with **stat-only churn** (zero content diff per `git diff`). I staged only the 4 real files; the binding "modifications" stayed in the working tree and did not enter the commit.
- The branch was created from `00de52b` but `origin/main` had moved to `9a7dc1f` (a docs commit on top) between worktree creation and push. I rebased cleanly before pushing — PR is a single commit on top of current `main`.

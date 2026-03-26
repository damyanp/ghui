# Plan: Sanitize Epic Inspection Workflow

## Background

`WorkItems::sanitize()` propagates an Epic field value down an issue hierarchy. When a child
item already has an Epic set — even if it conflicts with the Epic that should be inherited from
its ancestors — sanitize **skips** it and only logs a warning:

```rust
if this_item_epic.is_some() {
    warn!("{} - epic is '{}', should be '{}' - but not changing non-blank value", …);
} else {
    changes.add(Change { … ChangeData::Epic(epic.clone()) });
}
```

This means users have no way to see or act on these conflicts from the UI.
(Workstreams _are_ enforced unconditionally — the skip applies to Epics only.)

## Goal

Give the user a workflow to:
1. **Inspect** which issues have an Epic that sanitize would change (if not for the skip).
2. **Selectively approve** those overrides, staging them as normal pending Changes.

---

## Workflow Design

### Step 1 — Run Sanitize (unchanged behavior)

Clicking **Sanitize** works exactly as today: non-conflicting changes are staged, conflicting
Epics are still skipped.  After the command completes, if any Epic conflicts were detected, a
new **Conflicts** badge/count appears next to the Sanitize button (e.g. "3 epic conflicts").

### Step 2 — Open the Epic Conflicts Panel

Clicking the badge (or a separate **Review Epic Conflicts** button) opens a panel — modeled on
the existing changes/filter panels — that lists every conflict:

| Issue | Current Epic | Proposed Epic |
|-------|-------------|---------------|
| #123 Foo feature | Workstream A | Platform |
| #456 Bar task | (none set via `Some(x)` that differs) | Platform |

The panel has:
- A **Select All / Deselect All** toggle.
- Per-row checkboxes.
- A **Stage Selected** button that adds `ChangeData::Epic(proposed)` for the chosen items
  into the pending Changes (via the normal undo-tracked path), then closes the panel.

### Step 3 — Preview and Commit (unchanged behavior)

Once staged, the overrides behave like any other pending change: they appear in the diff, the
**Preview** toggle shows the hierarchy with the new Epics applied, and **Save** commits them
to GitHub.

---

## Implementation Plan

### 1. Backend: Richer sanitize return type (`github-graphql`)

**New public types in `github-graphql/src/data/work_items.rs`** (or a new
`sanitize_report.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SanitizeConflict {
    pub work_item_id: WorkItemId,
    pub current_epic: Option<FieldOptionId>,
    pub proposed_epic: FieldOptionId,   // always Some — only recorded when there is a conflict
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SanitizeReport {
    pub changes: Changes,
    pub epic_conflicts: Vec<SanitizeConflict>,
}
```

**Change `sanitize()` to return `SanitizeReport`** instead of `Changes`.  In the branch where
the skip currently happens, push a `SanitizeConflict` into `report.epic_conflicts`:

```rust
if this_item_epic.is_some() {
    // still skip — but record the conflict
    report.epic_conflicts.push(SanitizeConflict {
        work_item_id: id.clone(),
        current_epic: this_item_epic.clone(),
        proposed_epic: epic.clone().unwrap(),
    });
} else {
    report.changes.add(Change { … });
}
```

Update tests in `github-graphql/src/data/tests.rs` to assert on the new return type, and add
a test that exercises the conflict path.

### 2. Backend: App-layer changes (`ghui-app`)

**Update `DataState::sanitize()`** to:
- Call `work_items.sanitize(fields)` → `SanitizeReport`.
- Stage `report.changes` via `add_changes()` as before.
- Store `report.epic_conflicts` in `AppState` (new field `pending_epic_conflicts:
  Vec<SanitizeConflict>`), replacing any previous value.
- Return `(num_changes, num_conflicts)`.

**New method `DataState::stage_epic_overrides(ids: Vec<WorkItemId>)`**:
- Looks up each `id` in `pending_epic_conflicts`.
- Constructs `Change { data: ChangeData::Epic(Some(proposed_epic)) }` for each.
- Calls `add_changes()` (undo-tracked).
- Clears the staged items from `pending_epic_conflicts`.

**Update `Data` struct** to include `epic_conflicts: Vec<SanitizeConflict>` and push it to
the frontend as part of `DataUpdate::Data` (alongside `changes`, `nodes`, etc.).

### 3. Backend: Tauri command layer (`app/src-tauri`)

**Update `sanitize` command** — no signature change needed; the conflict list is pushed
automatically via the `DataUpdate` watcher.

**New command `stage_epic_overrides`**:
```rust
#[tauri::command]
pub async fn stage_epic_overrides(
    data_state: State<'_, DataState>,
    item_ids: Vec<WorkItemId>,
) -> TauriCommandResult<()> {
    data_state.stage_epic_overrides(item_ids).await?;
    Ok(())
}
```
Register in `tauri::generate_handler![…]`.

### 4. Frontend: Bindings and context (`app/src`)

- Run `cargo test --all` to regenerate TypeScript bindings for `SanitizeConflict` and
  `SanitizeReport` in `app/src/lib/bindings/`.
- Extend `WorkItemContext` / `Data` to expose `epicConflicts: SanitizeConflict[]`.
- Add `async stageEpicOverrides(ids: WorkItemId[])` that calls
  `invoke("stage_epic_overrides", { itemIds: ids })`.

### 5. Frontend: Epic Conflicts Panel component

New file `app/src/components/EpicConflictsPanel.svelte`:
- Receives `epicConflicts: SanitizeConflict[]` and `fields: Fields` as props.
- Renders a table: issue title/link, current epic name, proposed epic name, checkbox per row.
- "Select All" toggle, "Stage Selected" button (disabled when nothing selected).
- On submit: calls `context.stageEpicOverrides(selectedIds)` then closes.
- Styled consistently with existing panels (Skeleton UI / Tailwind 4 theme tokens).

### 6. Frontend: Toolbar integration

In `app/src/routes/+page.svelte`:
- After the **Sanitize** button, add a second **Epic Conflicts** button (using an
  appropriate Lucide icon, e.g. `GitBranch` or `AlertCircle`).
- Disabled when `epicConflicts.length === 0`.
- Shows a count badge when conflicts exist.
- `onclick` opens `EpicConflictsPanel`.
- Never hidden — visible but disabled when there are no conflicts (per toolbar conventions).

---

## Out of Scope

- Workstream conflicts: workstreams are already enforced unconditionally, so there is no
  "skip" to inspect.
- Batch-forcing all conflicts without inspection: the whole point is per-item selection.
- Persisting conflicts across app restarts: `pending_epic_conflicts` is in-memory only and
  cleared when the app loads fresh data.

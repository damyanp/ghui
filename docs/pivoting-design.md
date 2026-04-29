# Revamped Pivoting — Design Doc

> **Status:** Draft / discussion. This document is the artifact for issue
> *"Revamped pivoting"*. The goal is to flesh out concrete requirements and
> design options before we commit to an implementation. Mockups in this doc
> are intentionally low-fidelity (ASCII / Markdown) so they are easy to
> change as we iterate.

## 1. Background

ghui is a desktop tool for managing GitHub project items. The main view is a
tree of work items (issues / draft issues / PRs) that are part of a Project,
combined with the parent ↔ sub-issue hierarchy from GitHub.

Today the tree has a single, hardcoded notion of pivoting: at the top level
items are grouped by **Epic**. The relevant code lives in
[`ghui-app/src/nodes.rs`](../ghui-app/src/nodes.rs):

```rust
fn add_nodes(&mut self, items: &[WorkItemId], level: u32, path: &str) {
    let items = self.apply_filters(items);

    // For now, group by "Epic"
    let fn_get_group = |id| {
        self.work_items
            .get(id)
            .and_then(|item| item.project_item.epic.as_ref())
    };
    ...
}
```

Beyond that, `WorkItemStatistics.svelte` has a *separate, lightweight* pivot
mechanism for the stats panel (`PivotField = "kind" | "epic" | "workstream" |
"assigned" | "status"`, with optional series), but it does not affect the tree
and does not deal with hierarchy.

The relevant pivotable fields on a `WorkItem` today are:

| Field        | Source                      | Cardinality per item |
| ------------ | --------------------------- | -------------------- |
| `kind`       | Project single-select       | 0 or 1               |
| `epic`       | Project single-select       | 0 or 1               |
| `workstream` | Project single-select       | 0 or 1               |
| `iteration`  | Project iteration           | 0 or 1               |
| `status`     | Project single-select       | 0 or 1               |
| `assignees`  | Issue/PR field              | 0..N (multi-valued)  |
| `issueType`  | Issue field                 | 0 or 1               |
| `state`      | Issue/PR (open/closed/…)    | exactly 1            |
| `kind` of repo (`owner/name`) | derived         | exactly 1            |

## 2. Problem statement

We want a single, consistent pivoting concept that:

1. Lets the user **choose the field to pivot on** (not just Epic).
2. Plays well with the **issue parent/sub-issue hierarchy** — both when
   pivoting agrees with the hierarchy and when it does not.
3. Has a clear, predictable behaviour when **children of an item have
   different values for the pivoted field** (e.g. an Epic whose sub-issues
   are split across multiple Workstreams).
4. Is implementable without making `Changes` / `UndoHistory` aware of
   visualisation state (those layers stay pure).

Out of scope for this doc:

- The exact UI for *editing* values inline (we already have column menus,
  drag‑and‑drop, etc.).
- Saved views / per-user view persistence (mentioned briefly in §8).
- Multi-pivot / cross-tab visualisations beyond the existing stats panel.

## 3. Goals & non-goals

**Goals**

- G1. Single concept of "pivot field" applied consistently to:
  - the main tree (`WorkItemTree`)
  - the stats panel (`WorkItemStatistics`)
  - any future kanban / swimlane view
- G2. Sensible defaults so the tree continues to look familiar (default pivot
  remains *Epic*).
- G3. Expose enough information for the user to **see and resolve** items
  whose value disagrees with where they ended up grouped.
- G4. No regressions for filtering, drag-and-drop, change tracking, or undo.

**Non-goals**

- N1. Materialising a new persisted hierarchy server-side. Pivoting is
  purely a view concern.
- N2. Moving away from GitHub's parent/sub-issue model as the source of
  truth.

## 4. Current behaviour, in detail

### 4.1 Tree view (`NodeBuilder`)

- `NodeBuilder::add_nodes` is called with the project roots.
- It groups consecutive items by `project_item.epic` and, when there's more
  than one distinct value, emits a `NodeData::Group { name, field_option_id }`
  before each run.
- For each item it then recurses into `issue.sub_issues` — children are *not*
  re-grouped by Epic, they just inherit the Epic of their parent group.
- Sort order inside a group is the project's "ordered_items" order.

Implications:

- Children with a *different* Epic from their parent are simply rendered
  underneath their parent without any visual indication that they "belong"
  somewhere else in the Epic axis.
- The `sanitize` pass (`work_items.rs::sanitize_issue_hierarchy`) tries to
  push the parent's Epic onto its children, and reports `epic_conflicts` for
  items that already have a different Epic — so we know there's already a
  data model for "child disagrees with parent on Epic".

### 4.2 Stats panel

- `WorkItemStatistics.svelte` flattens `context.data.nodes` and pivots issues
  on user-chosen `rowPivotField` × `seriesPivotField`.
- It uses a `getPivotValues(issue, field)` helper that already returns a
  *list* of values per item — i.e. it's prepared for multi-valued pivots
  (assignees) by emitting one bucket per assignee.

This gives us a useful precedent: **a pivot value is logically a set of
strings/option-ids, not a single value.**

## 5. Concrete example scenarios

These are the scenarios designs in §6 must explain. They are intentionally
small but cover the awkward cases.

### Scenario A — Clean hierarchy, single pivot value

Pivot: **Epic**.

```
Project items                  Epic            Workstream
─────────────────────────────  ──────────────  ──────────
Epic: Performance              –               –
  Issue: Faster startup        Performance     Runtime
  Issue: Reduce memory         Performance     Runtime
Epic: Reliability              –               –
  Issue: Crash on shutdown     Reliability     Runtime
```

This is the case the current tree handles well.

### Scenario B — Mixed children (the headline problem)

Pivot: **Workstream**. The Epic "Performance" has children spread across
*Runtime* and *Tooling*.

```
Issue (parent)        sub-issue title           Workstream
────────────────────  ────────────────────────  ──────────
Epic: Performance     Faster startup            Runtime
                      Reduce memory             Runtime
                      Profiler dashboard        Tooling
                      CI perf benchmark         Tooling
                      Performance tracking spec (no value)
```

Open questions for B:

- Where does the Epic itself sort? It has *no* Workstream value of its own.
- Where does each child appear?
- If the user expands the Epic in the *Runtime* group, do they see only
  Runtime children, or all children?

### Scenario C — Multi-valued pivot (Assignees)

Pivot: **Assignee**. One issue is assigned to two people.

```
Issue                  Assignees
─────────────────────  ─────────────
Refactor field cache   alice, bob
Add status column      alice
Fix tooltip bug        carol
```

Open question: should "Refactor field cache" appear under both *alice* and
*bob*, or once under a synthetic *alice + bob* group?

### Scenario D — Pivoting across a deep tree

Pivot: **Status** on a 3-level tree.

```
Epic A (status = Active)
├── Story 1 (status = Active)
│   ├── Task a (status = Done)
│   └── Task b (status = Active)
└── Story 2 (status = Done)
    └── Task c (status = Done)
```

Open question: do we re-group at every level, only at the root, or somewhere
in between?

### Scenario E — Filtered + pivoted

Same data as Scenario B, but the user has filtered to `Workstream =
Tooling`. What does the *Performance* epic look like in that view?

## 6. Design options

These options are not mutually exclusive — option **6.4 (composable rules)**
is essentially "pick from 6.1–6.3 per situation".

For all options, the user picks the pivot field via a small toolbar control:

```
┌──────────────────────────────────────────────────────────────────┐
│  Group by: [ Epic ▾ ]   Sub-grouping: [ none ▾ ]   ☐ Show empty │
└──────────────────────────────────────────────────────────────────┘
```

`Group by` is the primary pivot (defaults to *Epic* to preserve current
behaviour). `Sub-grouping` is an optional secondary pivot — equivalent to
the existing `seriesPivotField` in the stats panel.

### 6.1 Option A — "Pivot first, hierarchy inside"

Items are flat-bucketed by their own pivot value at the top level. Within
each bucket, the parent/sub-issue hierarchy is preserved. Children whose
pivot value differs from their parent **do not move**; they stay under
their parent.

```
▾ Workstream: Runtime
  ▾ Epic: Performance               (Workstream: —)         ⓘ mixed children
      Faster startup                 (Workstream: Runtime)
      Reduce memory                  (Workstream: Runtime)
      Profiler dashboard             (Workstream: Tooling)  ⚠
      CI perf benchmark              (Workstream: Tooling)  ⚠
  Crash on shutdown                  (Workstream: Runtime)

▾ Workstream: Tooling
  (Performance epic appears here too — see §6.5)
```

Pros:
- Hierarchy is intuitive; you only have to learn one tree shape.
- Easy to map back to today's behaviour.

Cons:
- A parent can appear in a "wrong" bucket (the Epic above has no
  Workstream, but had to land somewhere). Needs the `ⓘ`/`⚠` annotations
  to make this discoverable.
- Items with mixed children can be misleading at a glance.

### 6.2 Option B — "Hierarchy first, pivot inside"

The top level is the existing root tree. *Within* each subtree, children
are grouped by the pivot value (similar to today's Epic behaviour, but the
field is configurable and applied at every level).

```
▾ Epic: Performance
  ▾ Workstream: Runtime
      Faster startup
      Reduce memory
  ▾ Workstream: Tooling
      Profiler dashboard
      CI perf benchmark
  ▾ Workstream: (none)
      Performance tracking spec
▾ Epic: Reliability
  ▾ Workstream: Runtime
      Crash on shutdown
```

Pros:
- Mixed children are *naturally* visible — they form distinct sub-groups.
- This is the smallest conceptual jump from current behaviour: today we
  group by Epic at the top; now we just allow the same pattern recursively
  with a configurable field.

Cons:
- For some pivots (status, assignee) you may want a flat view, not nested.
- Adds a level of indentation that doesn't always carry information (if a
  parent only has children with one value, the sub-group adds noise unless
  we collapse single-valued groups, which `NodeBuilder` already does — see
  `has_multiple_groups`).

### 6.3 Option C — "Flat pivot, ignore hierarchy"

Show items as a flat list bucketed only by the pivot value. The hierarchy
column shows the parent chain inline as breadcrumbs, but the tree itself
is one level deep.

```
▾ Workstream: Runtime    (4 items)
    Faster startup            ← Performance
    Reduce memory             ← Performance
    Crash on shutdown         ← Reliability
    Build cache invalidation  ← (no epic)
▾ Workstream: Tooling    (2 items)
    Profiler dashboard        ← Performance
    CI perf benchmark         ← Performance
▾ Workstream: (none)     (1 item)
    Performance tracking spec ← Performance
```

Pros:
- Best for "kanban-like" views (status, assignee, kind).
- Removes the mixed-children problem entirely — every item is bucketed by
  *its own* value.

Cons:
- Loses the sense of how work rolls up. We mitigate by showing the parent
  chain as a breadcrumb column.
- Probably wrong as the *only* default — users do want to see hierarchy.

### 6.4 Option D — Composable: a "view recipe"

Treat pivoting as a small ordered list of *axis rules*. The user picks
1..N axes; each axis is one of:

- `Group(field)` — bucket by this field (Option B, recursive)
- `Pivot(field)` — flat top-level bucket (Option A / C)
- `Hierarchy` — use the GitHub parent/sub-issue tree
- `Sort(field)` — sort within the current scope, no grouping

Today's behaviour is `[Pivot(Epic), Hierarchy]`. Common alternatives:

| Use case                       | Recipe                                  |
| ------------------------------ | --------------------------------------- |
| Today's Epic-first tree        | `Pivot(Epic) → Hierarchy`               |
| Hierarchy, sub-grouped         | `Hierarchy → Group(Workstream)`         |
| Flat status board              | `Pivot(Status)`                         |
| Per-assignee, per-epic         | `Pivot(Assignee) → Group(Epic)`         |
| Iteration plan                 | `Pivot(Iteration) → Hierarchy`          |

This is the most flexible model but also the most surface-area. Likely we
ship a curated set of presets first and expose the "custom recipe" UI
later.

### 6.5 Handling "mixed children"

Independent of which option we choose, we need a story for items whose
children disagree with them on the pivoted field. Candidates:

1. **Stay put with a badge.** Parent renders in its own bucket (or in
   `(none)` if it has no value). A small badge on the row indicates
   "children span N other groups". Hovering lists them.
   - Pros: zero duplication, fits the tree mental model.
   - Cons: easy to miss.

2. **Ghost rows.** The parent appears in *every* bucket that contains at
   least one of its descendants, but secondary copies are visually muted
   (italic/grey) and labelled "ghost — primary entry under <bucket>".
   - Pros: you can find the work no matter which axis value you picked.
   - Cons: needs careful handling of selection, drag-and-drop, and
     change-tracking (a change applied to a ghost must apply to the
     primary).

3. **"Mixed" synthetic group.** Items whose children disagree are placed
   into a synthetic `Mixed` bucket alongside the real values, and the
   children are listed underneath. Useful for Option B.
   - Pros: makes the disagreement extremely visible.
   - Cons: introduces a fake field value that doesn't exist on the server.

4. **Promote-children mode.** When the chosen pivot is set on the *child*
   level (e.g. Workstream typically lives on stories, not epics), break
   the parent ↔ child link in the view and bucket children by their own
   value. The parent still appears, but only above its same-bucket
   children.
   - This is essentially Option C applied selectively.

Recommended default: **(1) stay put with a badge**, with an opt-in toggle
for **(2) ghost rows** for users who want maximum visibility. **(3)** is
attractive for status/assignee but less so for hierarchy-defining fields
like Epic.

### 6.6 Multi-valued pivots (assignees)

For multi-valued fields, follow the precedent already set by
`getPivotValues` in `WorkItemStatistics.svelte`: an item with N values
appears in N buckets. To avoid double-counting in summary numbers we mark
all but the first occurrence as "secondary" (similar to ghost rows).

Unassigned items go into a `(unassigned)` bucket.

### 6.7 Empty groups

A "Show empty" toggle controls whether buckets with zero items are shown.
Default: off, except when filtering — then we still show empty buckets so
the user can see *why* their filter eliminated the items.

## 7. Mockups

### 7.1 Toolbar

```
┌──────────────────────────────────────────────────────────────────────┐
│ ghui  ▸ My Project                                   ⟲ undo  ⟳ redo │
├──────────────────────────────────────────────────────────────────────┤
│ Group by: [ Epic ▾ ] then [ none ▾ ]   View: [ Tree ▾ ]   ⚙        │
└──────────────────────────────────────────────────────────────────────┘
```

`View` lets us host alternative renderers in the future (Tree, Flat,
Board) sharing the same pivot configuration.

### 7.2 Tree, Option B (recommended default), Scenario B

```
▾ Epic: Performance
  ▾ Workstream: Runtime  (2)
      ▸ Faster startup            Active   alice
      ▸ Reduce memory             Active   bob
  ▾ Workstream: Tooling  (2)
      ▸ Profiler dashboard        Planned  carol
      ▸ CI perf benchmark         Planned  carol
  ▾ Workstream: (none)   (1)
      ▸ Performance tracking spec Planned  alice
▾ Epic: Reliability
  ▾ Workstream: Runtime  (1)
      ▸ Crash on shutdown         Active   bob
```

Note: when there is only *one* sub-group inside a parent, we collapse it
(reusing today's `has_multiple_groups` logic) so the noise stays
proportional to the actual disagreement.

### 7.3 Tree, Option A, Scenario B (with mixed badge)

```
▾ Workstream: Runtime  (3)
  ▾ Performance                        ⊕ mixed: 2 in Tooling, 1 in (none)
      Faster startup
      Reduce memory
  Crash on shutdown
▾ Workstream: Tooling  (2)
  Profiler dashboard       ↑ child of Performance
  CI perf benchmark        ↑ child of Performance
▾ Workstream: (none)  (1)
  Performance tracking spec ↑ child of Performance
```

The `⊕ mixed` badge is hover-expandable to a tooltip listing exactly which
children landed where, with click-to-jump.

### 7.4 Flat / Board, Option C, Scenario C (assignees)

```
┌──────────────┬────────────────────────────────┬──────────────┐
│ alice (3)    │ bob (2)                        │ carol (1)    │
├──────────────┼────────────────────────────────┼──────────────┤
│ Refactor *   │ Refactor field cache           │ Fix tooltip  │
│ Add status   │ Crash on shutdown              │              │
│ Faster start │                                │              │
└──────────────┴────────────────────────────────┴──────────────┘
   * = appears in another bucket too (assigned to bob)
```

(A board view is out of scope for the first cut, but the data shape
produced by the new pivoting layer should make it cheap to build.)

## 8. Interactions with other systems

- **Filters.** Filtering happens *before* pivoting, exactly as today.
  Empty buckets are hidden unless a filter is active (see §6.7).
- **Drag-and-drop.** When dragging an item between buckets in Option A/C,
  the drop should set the pivot field on the dragged item. In Option B,
  drops between sub-groups still mean "set the field" — drops between
  parent items still mean "reparent" as today. The two operations need
  to be visually distinguishable (different drop indicators).
- **Changes / UndoHistory.** No change. Pivoting is a pure view
  transform; it consumes the same `WorkItem` data and produces the same
  `Vec<Node>` shape (`NodeData::Group` already supports a `name` and a
  `field_option_id`, so additional pivot fields slot in naturally).
- **Sanitize rules.** Today `sanitize_issue_hierarchy` propagates Epic
  and Workstream from parent to child. With explicit pivoting, the
  "mixed children" badges become a UI surface for the same conflicts
  the sanitize pass already reports (`epic_conflicts`). We can offer a
  "sanitize from this view" action that uses the current pivot field as
  the propagation axis.
- **Stats panel.** Switch its `PivotField` enum to reuse the same enum we
  introduce for the tree, so "row" and "group by" are the same concept.

## 9. Implementation sketch

This section is non-binding; it just shows that the design is feasible
within the current architecture.

1. Introduce a `Pivot` value in Rust:

   ```rust
   pub enum PivotField {
       None,
       Epic,
       Workstream,
       Status,
       Kind,
       Iteration,
       Assignee,
       ItemType,     // GitHub issue/PR/draft
       Repository,
   }

   pub struct PivotConfig {
       pub primary: PivotField,
       pub secondary: PivotField, // None means "no sub-grouping"
       pub mixed_strategy: MixedStrategy, // Badge | Ghost | Mixed
   }
   ```

2. Generalise `NodeBuilder` so the "group by epic" closure becomes
   `fn group_key(field: PivotField, item: &WorkItem) -> Vec<Option<FieldOptionId>>`
   — note `Vec` to allow multi-valued pivots (assignees).

3. Plumb `PivotConfig` through `AppState` (it lives next to `Filters` —
   it is *not* part of `Changes`, matching the architectural rule that
   `Changes` is a pure data container).

4. Export `PivotField` / `PivotConfig` to TypeScript via `ts-rs`, like
   the existing field enums.

5. Replace the `WorkItemStatistics.svelte` pivot enum with the generated
   one so both surfaces stay in sync.

6. Default `PivotConfig` is `{ primary: Epic, secondary: None, mixed: Badge }`
   so nothing changes for existing users on first launch.

## 10. Open questions

- **Q1.** Should the pivot config be per-project, per-user-globally, or
  both? (Suggest: per-project, persisted in the same place as filters.)
- **Q2.** Do we want a "no grouping" mode (Option C with one bucket) as a
  first-class view, or is that just `View = Flat`?
- **Q3.** For Option B, should we re-group at *every* level of the tree,
  or only at the level immediately below the chosen pivot? (Today's code
  re-groups at every level by Epic, but the only difference Epic ever
  produces is at the root because children inherit.)
- **Q4.** Iteration is interesting because it's time-ordered. Do we sort
  iteration buckets chronologically by default and ignore the field's
  option order?
- **Q5.** Multi-valued pivot ghost rows: do edits to a ghost row modify
  the primary, or do we disable editing on ghosts to avoid surprises?

## 11. Recommendation

For an initial implementation:

1. Adopt **Option B** ("hierarchy first, pivot inside") as the default
   tree behaviour, parameterised by a chosen field. This is the smallest
   step from today and naturally surfaces mixed children as visible
   sub-groups.
2. Add a top-level "Group by" dropdown driven by `PivotField`; default to
   *Epic* so the existing experience is unchanged.
3. Use **mixed-strategy = Badge** initially (§6.5 option 1). Defer ghost
   rows until we have user feedback.
4. Reuse the same `PivotField` / `PivotConfig` in `WorkItemStatistics` so
   the stats and tree always agree on the available axes.
5. Defer Option C / board view and Option D / composable recipes to a
   follow-up, but keep the data layer (multi-valued group keys, mixed
   strategy enum) general enough to support them later.

## 12. Acceptance for this design exercise

This document is the deliverable for the issue. Concretely it should let
us:

- Decide which option to implement first (a recommendation is in §11).
- Open follow-up issues for: data-model changes (`PivotField`,
  generalised `NodeBuilder`), UI changes (toolbar, badges, mixed-mode
  toggle), and stats-panel reuse.
- Re-read this doc when the inevitable second wave of pivoting features
  shows up (board view, recipes, saved views).

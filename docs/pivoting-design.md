# Revamped Pivoting — Design Doc

> **Status:** Draft / discussion. This document is the artifact for issue
> *"Revamped pivoting"*. The goal is to flesh out concrete requirements and
> design options before we commit to an implementation. Mockups in this doc
> are intentionally low-fidelity (ASCII / Markdown) so they are easy to
> change as we iterate.
>
> All scenarios in this doc are taken from the **real project data** that
> `ghui-util get-all-items` produces from our HLSL working group project
> (titles, parent / sub-issue relationships, mixed-value patterns, and a
> real multi-assignee item are all genuine).

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
| Repository (`owner/name`) | derived         | exactly 1            |

## 2. Problem statement

We want a single, consistent pivoting concept that:

1. Lets the user **choose the field to pivot on** (not just Epic).
2. Lets the user **chain multiple pivots** (e.g. *Workstream → Epic*, or
   *Iteration → Workstream → Epic*) — pivoting is not just "primary +
   optional secondary", it's an ordered list of axes of arbitrary length.
3. Plays well with the **issue parent/sub-issue hierarchy** — both when
   pivoting agrees with the hierarchy and when it does not.
4. Has a clear, predictable behaviour when **children of an item have
   different values for the pivoted field** (e.g. an Epic whose sub-issues
   are split across multiple Workstreams — see the *DML Demo* and *Buffer
   Resources* scenarios in §5, both of which span 6–7 workstreams in the
   live data).
5. Is implementable without making `Changes` / `UndoHistory` aware of
   visualisation state (those layers stay pure).

Out of scope for this doc:

- The exact UI for *editing* values inline (we already have column menus,
  drag‑and‑drop, etc.).
- Saved views / per-user view persistence (mentioned briefly in §8).
- Cross-tab visualisations beyond the existing stats panel.

## 3. Goals & non-goals

**Goals**

- G1. Single concept of "pivot list" applied consistently to:
  - the main tree (`WorkItemTree`)
  - the stats panel (`WorkItemStatistics`)
  - any future kanban / swimlane view
- G2. Sensible defaults so the tree continues to look familiar (default
  pivot list remains `[Epic]`, matching today).
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

Today this is a parallel pivot system that doesn't share types or state with
the tree. We want to unify them.

## 5. Concrete example scenarios (real data)

These scenarios all use **real items** from our project (titles and parent
relationships pulled from the data the in-repo tooling fetches —
`ghui-util get-all-items` writes `all_items.json`). Field-option names
shown ("SM 6.10 (preview)", "DXIL Shader Flags", etc.) are the human
labels for the option IDs that appear in the raw data.

The three Epics referenced throughout are:

- **SM 6.10 (preview)**
- **SM 6.10 (retail)**
- **Alpha**

(Plus implicit `(none)` for items whose Epic field is unset.)

### Scenario A — Clean, single-pivot grouping (status quo)

Pivot list: **[Epic]**.

```
Project items                                       Epic              Workstream
──────────────────────────────────────────────────  ────────────────  ──────────
▾ Epic: SM 6.10 (retail)
    [Scenario] Buffer Resources                     SM 6.10 (retail)  (none)
    [HLSL] Add Root Signatures into DX Container    SM 6.10 (retail)  Root Sigs
▾ Epic: SM 6.10 (preview)
    [Scenario] Dynamic Resources                    SM 6.10 (preview) (none)
▾ Epic: Alpha
    Execution Tests for Long Vectors                Alpha             Long Vec
```

This is the case the current tree handles well — it is essentially what
ghui shows today.

### Scenario B — Mixed children (the headline problem)

Real example, untouched: **`[Scenario] DML Demo finalization`** (in
`llvm/wg-hlsl`). The parent's own Workstream is *DML demo*, its Epic is
*SM 6.10 (retail)*, and it has **55 direct sub-issues spread across 7
distinct Workstream values** (and many across multiple Epics too).

Pivot list: **[Workstream]**.

Sample of the sub-issues (real titles, abbreviated):

```
sub-issue                                                       Epic               Workstream
──────────────────────────────────────────────────────────────  ─────────────────  ─────────────
[HLSL] Clean up CodeGenHLSL/*-overload.hlsl tests               SM 6.10 (retail)   (none)
[HLSL] Invalid error reported for `int3` vector with packoffset SM 6.10 (retail)   Buffer
[HLSL][Sema] Misleading error with packoffset on a struct       SM 6.10 (retail)   Buffer
[DirectX] DXILCBufferAccess gets tripped up by 64-bit arrays    SM 6.10 (retail)   (none)
[HLSL][SPIRV] Clang should run spirv-val if available           SM 6.10 (retail)   (none)
[DirectX] crash during computeRegisterProperties                SM 6.10 (retail)   (none)
[DirectX] Legalize Lifetime markers                             SM 6.10 (retail)   DML demo
... +49 more, spanning a total of 7 Workstreams
```

Open questions for B:

- Where does the parent (`[Scenario] DML Demo finalization`) appear when
  the user pivots by Workstream? It has its *own* value (DML demo), but
  its children scatter across 7 buckets.
- If the user expands the parent in the *DML demo* bucket, do they see
  only the *DML demo* children, or all 55?
- How do we surface the disagreement so it's actionable, not invisible?

### Scenario C — Multi-valued pivot (Assignees), real example

Real example from the data: the issue **`[HLSL] Data race when writing to
independent elements of the same vector in TGSM`** is assigned to both
`bogner` and `hekota`. There are several other items assigned solely to
`bogner` and others solely to `hekota`.

Pivot list: **[Assignee]**.

```
Issue                                                                 Assignees
────────────────────────────────────────────────────────────────────  ─────────────────
[HLSL] Data race when writing to independent elements of vector TGSM  bogner, hekota
… (many other items assigned to bogner alone) …                       bogner
… (many other items assigned to hekota alone) …                       hekota
```

For this design we adopt the **synthetic combined-group** approach: the
multi-assigned item appears in a single `bogner + hekota` bucket alongside
the single-assignee `bogner` and `hekota` buckets, rather than being
duplicated into both individual buckets:

```
▾ Assignee: bogner          (N items)
    … bogner-only items …
▾ Assignee: bogner + hekota (1 item)
    [HLSL] Data race when writing to independent elements …
▾ Assignee: hekota          (M items)
    … hekota-only items …
```

Rationale (per review feedback): this avoids ghost-row mechanics, makes
counts add up cleanly, and keeps editing/undo simple — every item still
has exactly one place in the view.

Trade-off: in projects where most issues have many assignees, the long
tail of combined groups can become noisy. We accept this for the first
cut; a future toggle could swap to "explode into individual buckets" if
needed.

### Scenario D — Multi-pivot: *Workstream → Epic*

Pivot list: **[Workstream, Epic]**. Real data (sub-issues of
*`[Scenario] DML Demo finalization`*, just the *Buffer* Workstream slice):

```
▾ Workstream: Buffer
  ▾ Epic: SM 6.10 (retail)
      [HLSL] Invalid error reported for `int3` vector with packoffset
      [HLSL][Sema] Misleading error with packoffset on a struct or array
      [DirectX] Support typedBufferLoad and Store for RWBuffer<double2>
      [HLSL] Resource Arrays
      …
  ▾ Epic: (none)
      [HLSL] Buffer SRV type
▾ Workstream: DXIL Shader Flags
  ▾ Epic: SM 6.10 (retail)
      [DirectX] Implement Shader Flag Analysis for RequiresGroup
      [DirectX] Update DXContainerGlobals to get shader flags from metadata
  ▾ Epic: SM 6.10 (preview)
      [DirectX] Collect Shader Flags Mask based on Resource properties
  ▾ Epic: (none)
      [DirectX] Implement Shader Flag Analysis for `DX11_1_ShaderExtensions`
      [DirectX] Implement Shader Flag Analysis for `ViewID`
      …
```

This scenario is the headline use-case for the *multi-pivot* feature
requested in review. Because the headline pivot is *Workstream* (which
many of our items actually have set) and *Epic* is the secondary pivot,
the mixed-children problem from Scenario B mostly resolves itself — items
land in the bucket that matches their own value, and the secondary axis
makes Epic disagreements visible.

The pivot list can be longer:

- `[Iteration, Workstream, Epic]` — iteration plan with a workstream
  breakdown and Epic as a tertiary axis.
- `[Repository, Workstream]` — see how each repo splits across our
  workstreams (real repos in this project: `llvm/llvm-project` 813 items,
  `llvm/offload-test-suite` 207, `llvm/wg-hlsl` 184,
  `microsoft/DirectXShaderCompiler` 158, `microsoft/hlsl-specs` 81).

### Scenario E — Pivoting across a deep tree

Real example: **`[workstream] DXIL Shader Flags`** (in `llvm/wg-hlsl`) →
**`[DirectX] Collect Shader Flags Mask based on Instructions Used and
Shader Kind`** → **`[DirectX] Implement Shader Flag Analysis for
ResourceDescriptorHeapIndexing`** is a 3-deep chain. The intermediate
parent has 11 sub-issues spanning 2 Epics (some `SM 6.10 (preview)`, some
unset).

Pivot list: **[Epic]**.

Open question: do we re-group at every level, only at the root, or
somewhere in between? (Today's code re-groups at every level by Epic, but
because children inherit their parent's Epic the re-grouping below the
root is usually a no-op.)

### Scenario F — Filtered + pivoted

Same data as Scenario B, but the user has filtered to
`Workstream = Buffer`. What does the *DML Demo finalization* parent look
like in that view? In particular, does it appear at all (the parent's own
Workstream is `DML demo`, not `Buffer`), or only via its Buffer-tagged
children?

## 6. Design options

These options are not mutually exclusive — option **6.4 (composable
recipe)** is essentially "pick from 6.1–6.3 per situation".

For all options, the user picks the pivot list via a small toolbar
control:

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Group by:  [ Workstream ▾ ]  →  [ Epic ▾ ]  →  [ + ]    ☐ Show empty   │
└──────────────────────────────────────────────────────────────────────────┘
```

Each chip is one axis. `+` adds another axis; an `×` on each chip removes
it. Drag chips to reorder. The list `[]` means "no pivot, just hierarchy"
— equivalent to `View = Flat hierarchy`. The list `[Epic]` is today's
default.

### 6.1 Option A — "Pivot first, hierarchy inside"

Items are flat-bucketed by their own pivot value at the top level. Within
each bucket, the parent/sub-issue hierarchy is preserved. Children whose
pivot value differs from their parent **do not move**; they stay under
their parent.

Real-data preview, pivot `[Workstream]`:

```
▾ Workstream: DML demo                                  (1)
  ▾ [Scenario] DML Demo finalization                    ⊕ children: 7 ws
      [DirectX] Legalize Lifetime markers
▾ Workstream: Buffer                                    (4 items)
      [HLSL] Invalid error reported for `int3` vector with packoffset
      [HLSL][Sema] Misleading error with packoffset on a struct
      [HLSL] Resource Arrays
      [DirectX] Support typedBufferLoad/Store for RWBuffer<double2>
▾ Workstream: (none)                                    (many)
      …
```

Pros: hierarchy is intuitive; you only have to learn one tree shape.

Cons: the parent (`DML Demo finalization`) only appears once even though
its descendants span 7 buckets, so most descendants are detached from
their parent in the view. Needs the `⊕ children` badge to make that
discoverable. Becomes problematic when the multiplier is large (55 items
×  7 workstreams).

### 6.2 Option B — "Hierarchy first, pivot inside" *(recommended default)*

The top level is the existing root tree. *Within* each subtree, children
are grouped by the pivot list (single or multi). This is the smallest
conceptual jump from current behaviour: today we group by Epic at the
top; now we apply the same pattern recursively with a configurable list.

Pivot list `[Workstream, Epic]` applied to `[Scenario] DML Demo
finalization`:

```
▾ [Scenario] DML Demo finalization                      (55 sub-issues)
  ▾ Workstream: Buffer                                  (12)
    ▾ Epic: SM 6.10 (retail)                            (10)
        [HLSL] Invalid error reported for `int3` vector with packoffset
        [HLSL][Sema] Misleading error with packoffset on a struct
        …
    ▾ Epic: (none)                                      (2)
        [HLSL] Buffer SRV type
        …
  ▾ Workstream: DML demo                                (8)
    ▾ Epic: SM 6.10 (retail)
        [DirectX] Legalize Lifetime markers
        …
  ▾ Workstream: (none)                                  (35)
    ▾ Epic: SM 6.10 (retail)
        [HLSL] Clean up CodeGenHLSL/*-overload.hlsl tests
        [DirectX] DXILCBufferAccess gets tripped up by 64-bit arrays
        [HLSL][SPIRV] Clang should run spirv-val if available
        …
```

Pros:

- Mixed children are *naturally* visible — they form distinct sub-groups.
- Multi-pivot drops in cleanly: each axis adds one level of grouping
  inside the parent's subtree.
- Scales to N axes: `[Iteration, Workstream, Epic]` is just three nested
  levels under each parent.
- Smallest delta from existing behaviour and existing tests.

Cons:

- Adds a level of indentation that doesn't always carry information. We
  re-use today's `has_multiple_groups` logic to **collapse single-valued
  groups**: if every child of a parent has the same Workstream, no
  Workstream sub-group is rendered.
- For "kanban-style" pivots (status, assignee) you may want a flat view,
  not nested — see Option C.

### 6.3 Option C — "Flat pivot, ignore hierarchy"

Show items as a flat list bucketed only by the pivot list. The hierarchy
column shows the parent chain inline as breadcrumbs, but the tree itself
is one level deep per axis.

Pivot `[Assignee]`, drawn from real data:

```
▾ Assignee: spall              (96 items)
    [HLSL] Implement the `InstanceID` HLSL Function   ← Implement entire HLSL API set
    …
▾ Assignee: bogner             (39 items)
    …
▾ Assignee: bogner + hekota    (1 item)               ← synthetic combined group
    [HLSL] Data race when writing to independent elements …
▾ Assignee: hekota             (36 items)
    …
```

Pros: best for "kanban-like" views (status, assignee, kind). Removes the
mixed-children problem entirely — every item is bucketed by *its own*
value (or its synthetic combined value, see §6.6).

Cons: loses the sense of how work rolls up. We mitigate by showing the
parent chain as a breadcrumb column. Probably wrong as the *only*
default — users do want to see hierarchy.

### 6.4 Option D — Composable: a "view recipe"

Treat the entire view as a small ordered list of *axis rules*. Each axis
is one of:

- `Group(field)` — bucket by this field (Option B, recursive)
- `Pivot(field)` — flat top-level bucket (Option A / C)
- `Hierarchy` — use the GitHub parent/sub-issue tree
- `Sort(field)` — sort within the current scope, no grouping

Today's behaviour is `[Pivot(Epic), Hierarchy]`. Common alternatives:

| Use case                                | Recipe                                            |
| --------------------------------------- | ------------------------------------------------- |
| Today's Epic-first tree                 | `Pivot(Epic) → Hierarchy`                         |
| Hierarchy, sub-grouped by Workstream    | `Hierarchy → Group(Workstream)`                   |
| Hierarchy, sub-grouped Workstream→Epic  | `Hierarchy → Group(Workstream) → Group(Epic)`     |
| Flat status board                       | `Pivot(Status)`                                   |
| Per-assignee, per-epic                  | `Pivot(Assignee) → Group(Epic)`                   |
| Iteration plan, ws breakdown            | `Pivot(Iteration) → Group(Workstream) → Hierarchy`|
| Per-repo, per-workstream                | `Pivot(Repository) → Group(Workstream)`           |

This is the most flexible model but also the most surface area. The
`Group(...) → Group(...)` chains are the multi-pivot axis list described
in §2 / §6.2; `Pivot(...)` is the special case where the chain replaces
hierarchy at the root rather than nesting inside it.

We'd ship a curated set of presets first and expose the "custom recipe"
UI later. The recipe representation should still be the underlying data
model from day one so presets are just named recipes.

### 6.5 Handling "mixed children"

Independent of which option we choose, we need a story for items whose
children disagree with them on the pivoted field(s). Candidates:

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
     primary). Risky.

3. **"Mixed" synthetic group.** Items whose children disagree are placed
   into a synthetic `Mixed` bucket alongside the real values, and the
   children are listed underneath. Useful for Option B.
   - Pros: makes the disagreement extremely visible.
   - Cons: introduces a fake field value that doesn't exist on the
     server.

4. **Promote-children mode.** When the chosen pivot is set on the *child*
   level (e.g. Workstream typically lives on stories, not epics), break
   the parent ↔ child link in the view and bucket children by their own
   value. The parent still appears, but only above its same-bucket
   children.
   - This is essentially Option C applied selectively.

Recommended default: **(1) stay put with a badge** in Option B, since the
nested sub-grouping already makes the disagreement visible structurally.
Offer **(2) ghost rows** as an opt-in for users who want maximum
visibility in Option A. **(3)** is attractive for status/assignee but
less so for hierarchy-defining fields like Epic.

### 6.6 Multi-valued pivots (assignees) — synthetic combined groups

For multi-valued fields, **adopt synthetic combined groups**: an item
with N values appears in exactly one bucket whose key is the sorted set
of those N values (e.g. `bogner + hekota` from Scenario C).

This differs from the precedent set by `getPivotValues` in
`WorkItemStatistics.svelte`, which currently emits one row per
assignee. We accept that divergence intentionally — for the tree, "an
item only appears once" is a much stronger invariant than "every
assignee column sums to its real total". The stats panel can keep its
explode-by-assignee behaviour for charting, since charts have different
ergonomics.

Unassigned items go into a `(unassigned)` bucket. The synthetic combined
groups sort lexicographically by their joined name; an option in §10 is
to instead place each combined group directly after the alphabetically
smallest of its constituents.

### 6.7 Empty groups

A "Show empty" toggle controls whether buckets with zero items are
shown. Default: off, except when filtering — then we still show empty
buckets so the user can see *why* their filter eliminated the items.

## 7. Mockups

### 7.1 Toolbar

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ ghui  ▸ HLSL Working Group                                ⟲ undo  ⟳ redo    │
├──────────────────────────────────────────────────────────────────────────────┤
│ Group by:  [ Workstream ▾ ]  →  [ Epic ▾ ]  →  [ + ]   View: [ Tree ▾ ] ⚙  │
└──────────────────────────────────────────────────────────────────────────────┘
```

`View` lets us host alternative renderers in the future (Tree, Flat,
Board) sharing the same pivot list.

### 7.2 Tree, Option B (recommended default), pivot `[Workstream, Epic]`

Drawn from the live sub-issues of `[Scenario] DML Demo finalization`:

```
▾ [Scenario] DML Demo finalization                      55 sub-issues
  ▾ Workstream: Buffer                                  (12)
    ▾ Epic: SM 6.10 (retail)                            (10)   alice
        [HLSL] Invalid error for int3 vector with packoffset
        [HLSL][Sema] Misleading error with packoffset on a struct
        …
    ▾ Epic: (none)                                      (2)
        [HLSL] Buffer SRV type
  ▾ Workstream: DML demo                                (8)
    ▾ Epic: SM 6.10 (retail)
        [DirectX] Legalize Lifetime markers
  ▾ Workstream: (none)                                  (35)
    ▾ Epic: SM 6.10 (retail)
        [HLSL] Clean up CodeGenHLSL/*-overload.hlsl tests
        [DirectX] DXILCBufferAccess gets tripped up by 64-bit arrays
        [HLSL][SPIRV] Clang should run spirv-val if available
        …
```

Note: when there is only *one* sub-group inside a parent at a given
level, we collapse it (reusing today's `has_multiple_groups` logic) so
the noise stays proportional to the actual disagreement.

### 7.3 Tree, Option A, Scenario B (with mixed badge)

Pivot `[Workstream]`:

```
▾ Workstream: DML demo  (1)
  ▾ [Scenario] DML Demo finalization     ⊕ children: 7 workstreams (12 Buffer, 35 (none), …)
      [DirectX] Legalize Lifetime markers
▾ Workstream: Buffer    (12)
      [HLSL] Invalid error for int3 vector with packoffset    ↑ DML Demo finalization
      [HLSL][Sema] Misleading error with packoffset on a struct ↑ DML Demo finalization
      [HLSL] Resource Arrays                                    ↑ DML Demo finalization
      …
▾ Workstream: (none)   (35)
      [HLSL] Clean up CodeGenHLSL/*-overload.hlsl tests        ↑ DML Demo finalization
      …
```

The `⊕ children` badge is hover-expandable to a tooltip listing exactly
which children landed where, with click-to-jump.

### 7.4 Flat / Board, Option C, Scenario C (assignees with synthetic group)

```
┌──────────────┬──────────────────────────┬──────────────┬──────────────┐
│ bogner (39)  │ bogner + hekota (1)      │ hekota (36)  │ spall (96)   │
├──────────────┼──────────────────────────┼──────────────┼──────────────┤
│ …            │ [HLSL] Data race when    │ …            │ [HLSL] Impl. │
│              │ writing to independent   │              │  InstanceID  │
│              │ elements of vector TGSM  │              │ …            │
└──────────────┴──────────────────────────┴──────────────┴──────────────┘
```

(A board view is out of scope for the first cut, but the data shape
produced by the new pivoting layer should make it cheap to build.)

## 8. Interactions with other systems

- **Filters.** Filtering happens *before* pivoting, exactly as today.
  Empty buckets are hidden unless a filter is active (see §6.7).
- **Drag-and-drop.** When dragging an item between buckets in Option A/C,
  the drop should set the pivot field on the dragged item. With a
  multi-axis pivot, a drop targets the specific bucket the user dropped
  into and may need to set *multiple* fields at once (e.g. moving an
  item into `Workstream: Buffer / Epic: SM 6.10 (retail)` sets both
  fields). In Option B, drops between sub-groups still mean "set the
  field" — drops between parent items still mean "reparent" as today.
  The two operations need to be visually distinguishable (different drop
  indicators).
- **Multi-valued drops.** Dropping into a synthetic combined group
  (`bogner + hekota`) should *replace* the item's assignee set with the
  combined value, not append. This needs a confirmation dialog the first
  time.
- **Changes / UndoHistory.** No change. Pivoting is a pure view
  transform; it consumes the same `WorkItem` data and produces the same
  `Vec<Node>` shape (`NodeData::Group` already supports a `name` and a
  `field_option_id`, and we extend it with an axis tag and an optional
  set of field-option IDs to support combined groups).
- **Sanitize rules.** Today `sanitize_issue_hierarchy` propagates Epic
  and Workstream from parent to child. With explicit pivoting, the
  "mixed children" badges become a UI surface for the same conflicts
  the sanitize pass already reports (`epic_conflicts`). We can offer a
  "sanitize from this view" action that uses the current pivot list as
  the propagation order.
- **Stats panel.** Switch its `PivotField` enum to reuse the same enum
  we introduce for the tree, so "row" and "group by" are the same
  concept. The stats panel keeps its explode-by-assignee behaviour for
  charting; the tree uses combined groups (§6.6).

## 9. Implementation sketch

This section is non-binding; it just shows that the design is feasible
within the current architecture.

1. Introduce a `PivotField` enum and a `PivotConfig` value in Rust:

   ```rust
   pub enum PivotField {
       Epic,
       Workstream,
       Status,
       Kind,
       Iteration,
       Assignee,
       IssueType,    // GitHub-native issue type ("Bug", "Feature", …)
       WorkItemKind, // Issue / PullRequest / DraftIssue variant
       Repository,
       State,        // Open / Closed
   }

   pub struct PivotConfig {
       /// Ordered list of axes. Empty = no pivoting (pure hierarchy).
       /// Today's default = `vec![PivotField::Epic]`.
       pub axes: Vec<PivotField>,

       /// How to render a parent whose descendants disagree with it on
       /// one of the pivot axes.
       pub mixed_strategy: MixedStrategy,    // Badge | Ghost | Mixed

       /// How to render multi-valued items (assignees).
       pub multi_value_strategy: MultiValueStrategy, // Combined | Explode
   }
   ```

2. Generalise `NodeBuilder` so the "group by epic" closure becomes
   `fn group_key(field: PivotField, item: &WorkItem) -> GroupKey`,
   where `GroupKey` is either a single `Option<FieldOptionId>` for
   single-valued fields or a sorted `Vec<FieldOptionId>` for
   multi-valued fields with the `Combined` strategy.

3. Make `add_nodes` recurse over the *pivot axis list* before
   recursing into sub-issues, so that `[Workstream, Epic]` produces two
   nested `NodeData::Group` levels under each parent before the
   work-item rows.

4. Plumb `PivotConfig` through `AppState` (it lives next to `Filters` —
   it is *not* part of `Changes`, matching the architectural rule that
   `Changes` is a pure data container).

5. Export `PivotField` / `PivotConfig` to TypeScript via `ts-rs`, like
   the existing field enums.

6. Replace the `WorkItemStatistics.svelte` pivot enum with the
   generated one so both surfaces share field names; keep its
   explode-by-assignee behaviour locally for charting.

7. Default `PivotConfig` is `{ axes: vec![Epic], mixed: Badge,
   multi_value: Combined }` so nothing changes for existing users on
   first launch.

## 10. Open questions

- **Q1.** Should the pivot config be per-project, per-user-globally, or
  both? (Suggest: per-project, persisted in the same place as filters.)
- **Q2.** Do we want a "no grouping" mode (Option C with one bucket) as
  a first-class view, or is that just `View = Flat`?
- **Q3.** For Option B with multi-pivot, do we re-group at *every*
  level of the hierarchy, or only inside the immediate children of the
  pivoted parent?
- **Q4.** Iteration is interesting because it's time-ordered. Do we
  sort iteration buckets chronologically by default and ignore the
  field's option order?
- **Q5.** Sort order for synthetic combined groups (§6.6): join-name
  lexicographic, or "next to the alphabetically smallest constituent",
  or by item count?
- **Q6.** Should the toolbar show a small preview of the resulting
  bucket count as the user builds the pivot list (e.g. `Workstream (21)
  → Epic (5) ⇒ ~37 buckets`)?

## 11. Recommendation

For an initial implementation:

1. Adopt **Option B** ("hierarchy first, pivot inside") as the default
   tree behaviour, parameterised by a *list* of pivot fields. This is
   the smallest step from today, naturally surfaces mixed children as
   visible sub-groups, and supports multi-pivot (e.g. `[Workstream,
   Epic]`) without any extra model gymnastics.
2. Add a top-level "Group by" chip control driven by `PivotConfig`;
   default to `[Epic]` so the existing experience is unchanged.
3. Use **mixed-strategy = Badge** initially (§6.5 option 1). Defer
   ghost rows until we have user feedback.
4. Use **multi-value strategy = Combined** for assignees (§6.6, per
   review feedback). Defer the "explode" alternative.
5. Reuse the same `PivotField` / `PivotConfig` in `WorkItemStatistics`
   so the stats and tree always agree on the available axes; keep its
   explode-by-assignee behaviour locally for charting.
6. Defer Option C / board view and Option D / composable recipes to a
   follow-up, but keep the data layer (axis list, multi-valued group
   keys, mixed strategy enum) general enough to support them later.

## 12. Acceptance for this design exercise

This document is the deliverable for the issue. Concretely it should
let us:

- Decide which option to implement first (a recommendation is in §11).
- Open follow-up issues for: data-model changes (`PivotField`,
  `PivotConfig`, generalised `NodeBuilder`), UI changes (toolbar chips,
  badges, mixed-mode toggle), and stats-panel reuse.
- Re-read this doc when the inevitable second wave of pivoting features
  shows up (board view, recipes, saved views).

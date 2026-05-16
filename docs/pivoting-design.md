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
>
> ⚠️ The cached `all_items.json` checked into the repo was produced on
> 2026-04-27 and **may lag the live project by a few days**. Field-option
> *names* (e.g. *SM 6.10 (preview2)*, *DXIL Shader Flags*, …) come from
> the project's field configuration, not from the cached items file —
> they have been cross-referenced against the wording reviewers used in
> review comments rather than guessed. To regenerate from scratch, run
> `ghui-util get-all-items` locally with a PAT that has `project` scope;
> the sandbox that produced this PR can't reach `api.github.com` to
> refresh the file itself.
>
> A live, in-browser **prototype** lives at
> [`docs/pivoting-prototype.html`](./pivoting-prototype.html). It uses the
> **full project dataset (1,839 items, all field-option mappings)**
> embedded inline and implements **Option D — composable view recipes**
> (§6.4) on top of the rendering rules from §6.4a (ghost ancestors). The
> recipe is a free-form text expression (e.g. `Pivot(Epic) → Hierarchy`)
> with a preset dropdown and grammar help; users can type their own
> recipes and see the tree update immediately. Open the file in any
> browser to play with the trade-offs.
>
> ### Areas that need special attention
>
> Per review feedback (commit `6f74f7f` and follow-up), the following
> assumptions in earlier drafts have been retracted; reviewers should
> double-check that they are not silently relied on elsewhere in the
> document or in any follow-up issues:
>
> 1. **"Children inherit their parent's Epic in the current view."** They
>    don't — `NodeBuilder` re-groups at every level. See §4.1 for the
>    real behaviour and a concrete example.
> 2. **"`sanitize_issue_hierarchy` keeps parent and children aligned, so
>    we can lean on it as the data model."** A planned future change will
>    *intentionally* allow children to disagree with their parent on
>    Epic / Workstream. The pivoting design must therefore handle mixed
>    values directly, not assume sanitize will smooth them out. See
>    §3 N4.
> 3. **"We can unify the items view and the statistics view on a shared
>    `PivotField` enum."** That is now an explicit non-goal (§3 N3). The
>    items view stands on its own.
> 4. **Recommendation has shifted again.** Earlier drafts proposed
>    shipping four mutually-exclusive rendering modes (A / B / C / E) as
>    a single "View" picker. After building a working prototype against
>    the full dataset, the four modes turned out to be **special cases
>    of one composable model**: a small ordered list of axis rules
>    (`Pivot`, `Group`, `Hierarchy`, `Sort`) that can be expressed as a
>    short textual *recipe*. The current recommendation is therefore
>    **Option D (§6.4)** as the underlying data model and **a textual
>    recipe input + curated presets** as the first-iteration UI. The
>    older mode-picker framing is preserved in §6.1–§6.4a as the
>    catalogue of recipes worth shipping presets for. See §11 for the
>    rollout plan.

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
  - any future kanban / swimlane view

  The stats panel is explicitly **out** of this unification (N3).
- G2. Sensible defaults so the tree continues to look familiar (default
  pivot list remains `[Epic]`, matching today).
- G3. Expose enough information for the user to **see and resolve** items
  whose value disagrees with where they ended up grouped.
- G4. No regressions for filtering, drag-and-drop, change tracking, or undo.
- G5. **The four shapes from §6 (A / B / C / E) are expressible as
  recipes in one composable model** (Option D, §6.4). Different
  workflows favour different shapes; one rendering pipeline that
  handles all of them is cheaper to ship and maintain than four
  parallel modes. Concrete recipe equivalents are listed in §6.4.

**Non-goals**

- N1. Materialising a new persisted hierarchy server-side. Pivoting is
  purely a view concern.
- N2. Moving away from GitHub's parent/sub-issue model as the source of
  truth.
- N3. **Unifying the items view with the statistics view.** They have
  different ergonomics — the items view is a tree with editing and
  drag‑and‑drop; the stats view is a chart-oriented pivot. The two can
  evolve independently. (Earlier drafts of this doc proposed reusing a
  single `PivotField` enum across both; we now treat that as out of
  scope, per review feedback.)
- N4. **Relying on the current `sanitize_issue_hierarchy` propagation of
  Epic / Workstream from parent to child.** That pass is itself slated
  to be relaxed in a follow-up so children can legitimately disagree
  with their parents on these fields. The pivoting design must therefore
  treat **mixed values across a parent and its descendants as a
  first-class case**, not a hygiene violation that gets sanitized away.

## 4. Current behaviour, in detail

### 4.1 Tree view (`NodeBuilder`)

- `NodeBuilder::add_nodes` is called with the project roots and is then
  recursively re-entered for each item's `sub_issues`.
- At every level it groups consecutive items by `project_item.epic` and,
  when there's more than one distinct value, emits a
  `NodeData::Group { name, field_option_id }` before each run.
- Because the recursion re-applies the same Epic-grouping logic, **a
  parent whose children disagree on Epic already shows them as nested
  Epic groups under the parent** (not as a flat list). For example,
  `microsoft/DirectXShaderCompiler#7838` — whose direct children span
  *SM 6.10 (preview2)* and *SM 6.10 (retail)* — currently renders as:

  ```
  ▾ #7838  Implement HLSL `__builtin` intrinsics and DXIL Ops
    ▾ Epic: SM 6.10 (preview2)
        #8271 …
        #8416 …
    ▾ Epic: SM 6.10 (retail)
        #8270 …
        #8313 …
        …
  ```

- Sort order inside a group is the project's "ordered_items" order; the
  Epic option order comes from `Fields::epic.option_index`.

Implications:

- The hardcoded axis is **Epic**. Picking a different axis (Workstream,
  Status, Iteration, …) requires a code change. There is no UI surface
  for it.
- The grouping happens *only* on the Epic axis. If we want a multi-axis
  pivot (e.g. *Workstream → Epic*), this code can't express it.
- Today's behaviour for "child disagrees with parent on Epic" is the
  nested-groups rendering above. That's a reasonable answer for one
  axis, but it doesn't extend obviously to "children disagree across
  multiple axes" or to non-Epic fields, and it gives no top-level
  visibility — you only see the disagreement after expanding the
  parent.

### 4.2 Stats panel

- `WorkItemStatistics.svelte` flattens `context.data.nodes` and pivots issues
  on user-chosen `rowPivotField` × `seriesPivotField`.
- It uses a `getPivotValues(issue, field)` helper that already returns a
  *list* of values per item — i.e. it's prepared for multi-valued pivots
  (assignees) by emitting one bucket per assignee.

The stats panel is **out of scope** for this design (see §3 N3); it's
described here only because it gives a useful precedent for multi-valued
pivots, not because we plan to share code or types with it.

## 5. Concrete example scenarios (real data)

These scenarios all use **real items** from our project (titles and parent
relationships pulled from the data the in-repo tooling fetches —
`ghui-util get-all-items` writes `all_items.json`). Field-option names
shown ("SM 6.10 (preview2)", "DXIL Shader Flags", etc.) are the human
labels for the option IDs that appear in the raw data.

The three Epics referenced throughout are:

- **SM 6.10 (preview2)**
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
▾ Epic: SM 6.10 (preview2)
    [Scenario] Dynamic Resources                    SM 6.10 (preview2) (none)
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
  ▾ Epic: SM 6.10 (preview2)
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
parent has 11 sub-issues spanning 2 Epics (some `SM 6.10 (preview2)`, some
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

### 6.2 Option B — "Hierarchy first, pivot inside"

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

### 6.4 Option D — Composable: a "view recipe" *(recommended)*

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

**Options A, B, C, and E are all special cases of this model:**

| Earlier option                       | Equivalent recipe                                 |
| ------------------------------------ | ------------------------------------------------- |
| Option A — Pivot first, no ghosts    | `Pivot(F)` *(no `Hierarchy` axis)*                |
| Option B — Hierarchy first, sub-pivot| `Hierarchy → Group(F)`                            |
| Option C — Flat                      | `Pivot(F)` with no further axes                   |
| Option E — Pivot at top + ghosts     | `Pivot(F) → Hierarchy` *(ghost ancestors on)*     |

The ghost-row rendering rules from §6.4a apply uniformly whenever a
`Hierarchy` axis sits *inside* a `Pivot(...)` or `Group(...)` axis: any
ancestor on the path from a primary descendant up to the bucket root
that is not itself a primary member is rendered as a (muted, italic,
non-interactive) ghost. This is a single rendering rule, not four.

#### 6.4.1 Textual recipe grammar (first-iteration UI)

The first UI iteration is a single text input that accepts a recipe in
this grammar:

```
recipe := axis (SEP axis)*
SEP    := "→" | "->" | ">" | ","
axis   := "Pivot" "(" field ")"
        | "Group" "(" field ")"
        | "Sort"  "(" field ")"
        | "Hierarchy"
field  := Status | Blocked | Epic | Iteration | Kind
        | Workstream | Estimate | Priority
        | Assignee | Repository | Type | State
```

Parsing is case-insensitive on keywords and field names; whitespace is
ignored; common aliases (`Repo`, `Assignees`, `Owner`) are accepted.
When the `Hierarchy` axis is hit, the remaining axes in the recipe are
recursively re-applied to the children of each level of the source
tree, so `Pivot(Epic) → Hierarchy → Group(Status)` means "top-level
buckets by Epic, then the parent/child tree, with each level's
children sub-grouped by Status".

The UI ships:

1. A free-form recipe text input with grammar help in a collapsible.
2. A dropdown of curated presets that just write into the text input
   (so users can edit them as a starting point).
3. A handful of orthogonal toggles that don't belong in the recipe:
   *show counts*, *collapse single-valued groups*, *hide closed*,
   *explode multi-valued (assignees)*, *show ghost ancestors*.

The textual recipe is a stepping stone, not the final UI: once the
shape settles, presets get first-class names and a future iteration can
add a chip-style visual builder on top of the same parser. The
*underlying data model is the recipe from day one*, so any future UI is
just a different editor over the same `Vec<Axis>`.

A working prototype of this model against the full 1,839-item dataset
is at [`docs/pivoting-prototype.html`](./pivoting-prototype.html).

### 6.4a Option E — "Pivot at top with ghost rows" *(default mode)*

Suggested in review (comment on §6 of the previous draft). Always group
at the top by the pivot list. Within each pivot bucket, render the
hierarchy of items whose own pivot value matches the bucket — but where
a matching child has a parent *not* in this bucket, render the parent
too as a **ghost row** so the child stays visually attached to it.
Ghost rows are visually muted and labelled; the same parent may appear
ghosted in several buckets.

**Rendering rules** (refined while building the prototype):

1. **Bucket membership** is built in two passes. First pass: an item
   is a *primary* member of bucket `B` iff its own pivot value(s)
   match `B`. Second pass: for every primary member, walk its ancestor
   chain in the source tree and add each ancestor to `B` as a *ghost*
   member if it is not already a primary there. The pivot value of a
   ghost is irrelevant — its only job is to attach a primary
   descendant to its real parent.
2. **Bucket roots** are members of `B` whose parent in the source tree
   is *not* in `B`. Each bucket is rendered as a forest of those
   roots, recursing only into children that are also members of `B`
   (primary or ghost).
3. **Ghost styling** is muted (italic, a `(ghost)` prefix, a `(ghost)`
   badge or similar). Counts are reported as
   "*N primary* + *M ghost*" so it is unambiguous which figure to
   trust for "how much work is in this bucket".
4. **Multi-axis pivots collapse repeated leading headers.** When the
   pivot list has more than one axis (e.g. `[Epic, Workstream]`), the
   header rows for two consecutive buckets that share the same Epic
   value emit the Epic header *only once*, followed by the differing
   Workstream sub-headers underneath. This avoids the "Epic: (none) /
   Workstream: (none) / Epic: (none) / Workstream: DXIL …" repetition
   that an earlier prototype build produced.
5. **Multi-valued items** (e.g. assignees) follow the chosen
   multi-value strategy from §6.6 — under *Combined*, an item lands
   primary in exactly one synthetic bucket; under *Explode*, it lands
   primary in each constituent value's bucket. Either way, ghost
   ancestors are added per (1).
6. **Interaction routing.** A click / drag / edit on a ghost routes to
   the primary occurrence of that item; the ghost itself is
   non-interactive. Change-tracking markers (e.g. "modified") still
   render on ghosts because they reflect the underlying item's state.

Abstract example (verbatim from review):

| Parent | Item | Pivot Field |
| ------ | ---- | ----------- |
|  —     | A    | X           |
|  —     | B    | Y           |
| A      | AA   | X           |
| A      | AB   | Y           |
| B      | BA   | X           |
| B      | BB   | Y           |

Pivot list: **[Pivot Field]**:

```
▾ X
  ▾ A
      AA
  ▾ (B)         ← ghost: B's pivot value is Y, but BA lives in X
      BA
▾ Y
  ▾ (A)         ← ghost: A's pivot value is X, but AB lives in Y
      AB
  ▾ B
      BB
```

`(B)` and `(A)` are ghosts: they appear in a bucket where their own
value disagrees, purely so their child stays attached. They are not
selectable for editing, dragging, or change-tracking — any action on a
ghost row routes to its primary entry in the matching bucket.

Real-data example, pivot `[Epic]`, using
**`/llvm/llvm-project/issues/116143`** ("[DirectX] Collect Shader Flags
Mask based on Resource properties in the Shader", Epic = *SM 6.10
(preview2)*) and its 4 direct sub-issues, two of which have Epic = *SM
6.10 (preview2)* and two of which have no Epic set:

```
▾ Epic: SM 6.10 (preview2)              2 primary
  ▾ [DirectX] Collect Shader Flags Mask based on Resource properties …
      [DirectX] Implement Shader Flags Analysis for `AtomicInt64OnTypedResource`
      [DirectX] Implement Shader Flags Analysis for `AtomicInt64OnHeapResource`
▾ Epic: (none)                          2 primary + 1 ghost
  ▾ ([DirectX] Collect Shader Flags Mask based on Resource properties …)   ← ghost
      [DirectX] Implement Shader Flags Analysis for `AtomicInt64OnGroupShared`
      [DirectX] Implement shader flag analysis for EnableRawAndStructuredBuffers
```

Multi-axis example, pivot `[Epic, Workstream]` over the same family —
note the leading `Epic:` header is emitted once per Epic, with
Workstream sub-headers nested underneath it (rule 4 above):

```
▾ Epic: SM 6.10 (preview2)
  ▾ Workstream: DXIL Shader Flags        2 primary
    ▾ [DirectX] Collect Shader Flags Mask …
        [DirectX] Implement Shader Flags Analysis for `AtomicInt64OnTypedResource`
        [DirectX] Implement Shader Flags Analysis for `AtomicInt64OnHeapResource`
▾ Epic: (none)
  ▾ Workstream: (none)                   2 primary + 1 ghost
    ▾ ([DirectX] Collect Shader Flags Mask …)   ← ghost
        [DirectX] Implement Shader Flags Analysis for `AtomicInt64OnGroupShared`
        [DirectX] Implement shader flag analysis for EnableRawAndStructuredBuffers
```

The same pattern with the larger Scenario B parent (*[Scenario] DML Demo
finalization*, Epic = *SM 6.10 (retail)*, 55 sub-issues spanning multiple
Epics): the parent appears as primary under *SM 6.10 (retail)* with the
matching children, and as a ghost row under each other Epic that owns at
least one child, with just those children listed beneath. With a
multi-axis pivot `[Workstream, Epic]`, ghosts can appear at any of the
nested levels — the parent is ghosted into each `(Workstream, Epic)`
bucket whose contents include one of its descendants.

Pros:

- The pivot axis is always dominant — buckets contain only items whose
  own value matches.
- Mixed-children parents become *highly* visible because they appear in
  multiple buckets simultaneously.
- Works uniformly for any pivot field, single or multi-axis, without
  caring whether the field "naturally" lives on parents or on children.
- Doesn't depend on sanitize behaviour at all (§3 N4).

Cons / things to design carefully:

- **Duplication.** A parent with descendants in N buckets appears N
  times. Counts at the bucket headers must be unambiguous: we'll show
  *primary* counts (items whose own value matches) and not double-count
  ghost rows.
- **Editing & drag-and-drop on ghosts.** Ghosts must be obviously
  non-editable. A drag of an item out of its bucket means "set the
  pivot value"; a drag *of a ghost* must be disabled (or routed to the
  primary).
- **Change tracking / undo.** A ghost row showing a modified item
  should still display the modified marker, but `Changes` is keyed on
  the underlying `WorkItemId` so this is purely a render concern.
- **Selection.** Single-click on a ghost should focus the primary
  occurrence (jump to it) rather than treat the ghost as the
  selection.
- **Performance.** Worst case the rendered node count is `O(items × N
  buckets per parent)`. In practice the ghost duplication is bounded
  by the parent's own descendant count, but this needs to be measured
  against our largest scenarios (DML Demo finalization × 7 Workstream
  buckets ≈ 60 extra nodes; manageable).

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

Recommended default: **(2) ghost rows** as embodied by Option E (§6.4a).
Ghost rows are the suggestion explicitly raised in review and they keep
the pivot axis dominant for every field (Epic, Workstream, Iteration,
…) without depending on whether the field "naturally" lives on parents
or on children. **(1) Stay-put + badge** is offered as a lower-density
alternative for Option B users. **(3) Mixed bucket** is attractive for
status/assignee but less so for hierarchy-defining fields like Epic, and
**(4) Promote-children** is essentially a special case of Option C.

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
│ Group by: [ Epic ▾ ] → [ Workstream ▾ ] → [ + ]   View: [ Pivot+ghosts ▾ ] ⚙│
└──────────────────────────────────────────────────────────────────────────────┘
```

`Group by` is the pivot-axis list (`PivotConfig::axes`). `View` is
`PivotConfig::mode` and offers the four §6 options:

- **Pivot + ghosts** (default, Option E)
- **Hierarchy first** (Option B)
- **Pivot, no ghosts** (Option A)
- **Flat** (Option C)

Switching mode preserves the axis list and any filters.

### 7.2 Tree, Option B (alternative mode), pivot `[Workstream, Epic]`

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
  and Workstream from parent to child. **A planned future change will
  relax that** so children may legitimately disagree with their parent
  (§3 N4). The pivoting design therefore must not assume the sanitize
  pass will keep things aligned: ghost rows / badges have to work
  correctly on data where mixed values are the norm, not the
  exception.
- **Stats panel.** Out of scope (§3 N3). The stats panel keeps its
  current implementation; we are deliberately *not* sharing types with
  it in this round.

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
       /// Ordered list of axes (Option D, §6.4). Empty = flat list of
       /// all matching items, no grouping or hierarchy.
       /// Today's default = `Pivot(Epic) → Hierarchy`.
       pub recipe: Vec<Axis>,

       /// How to render multi-valued items (assignees).
       pub multi_value_strategy: MultiValueStrategy, // Combined | Explode

       /// Render ancestors of primary items as muted ghost rows
       /// whenever a `Hierarchy` axis sits inside a `Pivot` or
       /// `Group` axis (§6.4a rules 1–6). On by default.
       pub show_ghost_ancestors: bool,
   }

   pub enum Axis {
       /// Top-level bucket by this field. The remaining axes apply
       /// inside each bucket.
       Pivot(PivotField),
       /// Recursive sub-bucket inside the current scope.
       Group(PivotField),
       /// Use the GitHub parent/sub-issue tree. Remaining axes (if
       /// any) are re-applied to the children of each level.
       Hierarchy,
       /// Sort the current scope by this field; no grouping.
       Sort(PivotField),
   }
   ```

2. Generalise `NodeBuilder` so the "group by epic" closure becomes
   `fn group_key(field: PivotField, item: &WorkItem) -> GroupKey`,
   where `GroupKey` is either a single `Option<FieldOptionId>` for
   single-valued fields or a sorted `Vec<FieldOptionId>` for
   multi-valued fields with the `Combined` strategy.

3. Make `add_nodes` interpret the recipe directly. The pipeline walks
   the axis list once: each `Pivot` / `Group` axis buckets the current
   scope by its field, each `Hierarchy` axis renders the parent /
   sub-issue tree of the items in scope (with ghost ancestors when
   `show_ghost_ancestors` is true) and re-applies any remaining axes
   to each level's children, and `Sort` axes order the current scope
   without bucketing. The four "modes" from §6.1–§6.4a fall out as
   special cases:

   - Option B (Hierarchy first, sub-pivot) → `Hierarchy → Group(F)`.
   - Option E (Pivot at top + ghosts) → `Pivot(F) → Hierarchy` with
     `show_ghost_ancestors = true`.
   - Option A (Pivot at top, no ghosts) → `Pivot(F) → Hierarchy` with
     `show_ghost_ancestors = false` (mixed-parent badge per §6.5).
   - Option C (Flat) → `Pivot(F)` with no `Hierarchy` axis.

   The ghost rules from §6.4a (rule 1: primary + ghost two-pass
   membership; rule 2: bucket roots; rule 4: coalesce repeated leading
   axis headers across consecutive buckets) apply uniformly wherever a
   `Hierarchy` axis is nested inside a `Pivot` or `Group`.

4. Plumb `PivotConfig` through `AppState` (it lives next to `Filters` —
   it is *not* part of `Changes`, matching the architectural rule that
   `Changes` is a pure data container).

5. Export `PivotField` / `Axis` / `PivotConfig` to TypeScript via
   `ts-rs`, like the existing field enums.

6. Default `PivotConfig` is `{ recipe: [Pivot(Epic), Hierarchy],
   show_ghost_ancestors: true, multi_value: Combined }`. This is
   *very close* to today's behaviour for items whose Epic agrees with
   their parent's, and it makes the existing mixed-Epic cases (e.g.
   DXC#7838) more visible by hoisting them into the matching
   top-level Epic bucket. Users who prefer today's parent-centric
   shape can write `Hierarchy → Group(Epic)` (or pick that preset);
   teams driving roadmap reviews from the doc can write
   `Pivot(Status)` for a flat board.

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

1. **Adopt Option D (§6.4) as the underlying data model.** A
   `PivotConfig` is a `Vec<Axis>` where each axis is `Pivot(field)`,
   `Group(field)`, `Hierarchy`, or `Sort(field)`. The four
   mutually-exclusive "modes" from §6.1–§6.4a (A / B / C / E) become
   *recipes*, not modes. This keeps the implementation tractable
   (one rendering pipeline, not four) while letting users compose
   shapes the original mode picker would not have produced (e.g.
   `Pivot(Status) → Group(Workstream) → Hierarchy`).
2. **Ship a textual recipe input as the first UI** (§6.4.1), backed by
   a curated set of presets that simply write into the text input.
   The default recipe is `Pivot(Epic) → Hierarchy`, which is
   identical in behaviour to today's tree (Option E with Epic). The
   recipe parser, grammar, and 14 working presets are demonstrated in
   [`docs/pivoting-prototype.html`](./pivoting-prototype.html).
3. **Ghost ancestors are always-on whenever `Hierarchy` sits inside a
   `Pivot(...)` or `Group(...)`** (§6.4a rules 1–6), with a single
   toggle to hide them for users who prefer the cleaner Option A
   look. There is no separate "mode" enum.
4. **Multi-value strategy = Combined** for assignees by default
   (§6.6, per review feedback). The *Explode* alternative is a single
   toggle that lives next to the recipe input, not part of the recipe
   itself.
5. **Persist the `PivotConfig` (recipe text + toggles) per project**,
   in the same place as filters. Saved recipes get first-class names
   in a follow-up iteration.
6. **Statistics view stays out of scope** (§3 N3) — it keeps its
   current independent implementation.
7. **Defer a chip-style visual recipe builder** to a follow-up. The
   textual input + presets is the smallest thing that exposes the
   full power of the data model; once the shape stabilises, a richer
   editor can be layered on top of the same parser.

A live in-browser prototype demonstrating this model against the full
1,839-item dataset is at
[`docs/pivoting-prototype.html`](./pivoting-prototype.html). It
implements the textual recipe parser, the curated presets, the
orthogonal toggles, and the ghost-ancestor rendering rules from
§6.4a — use it to sanity-check the recommendation against real data
before we commit.

## 12. Acceptance for this design exercise

This document is the deliverable for the issue. Concretely it should
let us:

- Decide which option to implement first (a recommendation is in §11).
- Open follow-up issues for: data-model changes (`PivotField`,
  `PivotConfig`, generalised `NodeBuilder`) and UI changes (toolbar
  chips, ghost rendering, mixed-mode toggle).
- Re-read this doc when the inevitable second wave of pivoting features
  shows up (board view, recipes, saved views).

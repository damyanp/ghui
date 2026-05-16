export const PRESETS: ReadonlyArray<{ label: string; recipe: string }> = [
  {
    label: "Today's tree (Epic, then hierarchy)",
    recipe: "Pivot(Epic) → Hierarchy",
  },
  {
    label: "Hierarchy first, sub-grouped by workstream",
    recipe: "Hierarchy → Group(Workstream)",
  },
  {
    label: "Hierarchy first, sub-grouped by status",
    recipe: "Hierarchy → Group(Status)",
  },
  { label: "Flat status board", recipe: "Pivot(Status)" },
  {
    label: "Flat status × workstream",
    recipe: "Pivot(Status) → Group(Workstream)",
  },
  {
    label: "Per-assignee, per-epic",
    recipe: "Pivot(Assignee) → Group(Epic)",
  },
  { label: "Iteration plan", recipe: "Pivot(Iteration) → Hierarchy" },
  {
    label: "Repository, then epic, then hierarchy",
    recipe: "Pivot(Repository) → Group(Epic) → Hierarchy",
  },
  {
    label: "Issue-type breakdown",
    recipe: "Pivot(IssueType) → Group(Status)",
  },
  { label: "Open vs closed", recipe: "Pivot(State) → Group(Epic)" },
  { label: "Hierarchy only (parent ↔ sub-issues)", recipe: "Hierarchy" },
  {
    label: "Sort by priority, then tree",
    recipe: "Sort(Priority) → Hierarchy",
  },
  { label: "Just sort by epic (no grouping)", recipe: "Sort(Epic)" },
  { label: "PRs vs Issues", recipe: "Pivot(Type) → Group(Status)" },
];

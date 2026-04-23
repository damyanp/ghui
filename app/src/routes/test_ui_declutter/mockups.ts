export type ToolbarItem = {
  label: string;
  group: "load" | "edit" | "review" | "modes" | "system";
  count?: number;
  active?: boolean;
};

export type MockScenario = {
  id: string;
  title: string;
  context: string;
  currentToolbar: ToolbarItem[];
  proposedToolbar: ToolbarItem[];
  currentFlow: string[];
  proposedFlow: string[];
};

export const scenarios: MockScenario[] = [
  {
    id: "read-only",
    title: "Read-only browsing",
    context: "Typical session with refresh, scanning, and no outbound changes.",
    currentToolbar: [
      { label: "Refresh", group: "load" },
      { label: "Save", group: "edit" },
      { label: "Discard", group: "edit" },
      { label: "Details", group: "review" },
      { label: "Preview", group: "review" },
      { label: "Add", group: "edit" },
      { label: "Sanitize", group: "edit" },
      { label: "Epic Conflicts", group: "review" },
      { label: "Undo", group: "edit" },
      { label: "Redo", group: "edit" },
      { label: "Items", group: "modes", active: true },
      { label: "X-tracker", group: "modes" },
      { label: "Statistics", group: "modes" },
      { label: "Output", group: "system" },
      { label: "Updates", group: "system" },
      { label: "Pat", group: "system" },
    ],
    proposedToolbar: [
      { label: "Refresh", group: "load" },
      { label: "Modes", group: "modes", active: true },
      { label: "Find", group: "review" },
      { label: "Output", group: "system" },
      { label: "Updates", group: "system" },
      { label: "Pat", group: "system" },
      { label: "More actions", group: "edit" },
    ],
    currentFlow: ["Refresh", "Scan tree", "Open Statistics", "Output if needed"],
    proposedFlow: [
      "Refresh",
      "Browse (Items/Statistics in compact mode selector)",
      "Quick Find",
      "More actions for low-frequency edit controls",
    ],
  },
  {
    id: "editing",
    title: "Active editing and save",
    context: "A working session with many pending changes and a visible save step.",
    currentToolbar: [
      { label: "Refresh", group: "load" },
      { label: "Save", group: "edit" },
      { label: "Discard", group: "edit" },
      { label: "Details", group: "review", count: 18 },
      { label: "Preview", group: "review" },
      { label: "Add", group: "edit" },
      { label: "Sanitize", group: "edit" },
      { label: "Epic Conflicts", group: "review", count: 2 },
      { label: "Undo", group: "edit" },
      { label: "Redo", group: "edit" },
      { label: "Items", group: "modes", active: true },
      { label: "Output", group: "system", count: 1 },
      { label: "Updates", group: "system" },
      { label: "Pat", group: "system" },
    ],
    proposedToolbar: [
      { label: "Refresh", group: "load" },
      { label: "Sanitize", group: "edit" },
      { label: "Review Changes", group: "review", count: 18 },
      { label: "Conflicts", group: "review", count: 2 },
      { label: "Save", group: "edit", active: true },
      { label: "Output", group: "system", count: 1 },
      { label: "More actions", group: "edit" },
      { label: "Pat", group: "system" },
    ],
    currentFlow: [
      "Change fields in table",
      "Optionally open Details",
      "Save (progress in button background)",
    ],
    proposedFlow: [
      "Edit inline",
      "Review Changes (single panel for details + preview + conflicts)",
      "Save (sticky and high-emphasis)",
    ],
  },
  {
    id: "cleanup",
    title: "Conflict-heavy cleanup",
    context: "Session with sanitize + conflict resolution before final save.",
    currentToolbar: [
      { label: "Sanitize", group: "edit" },
      { label: "Epic Conflicts", group: "review", count: 7 },
      { label: "Details", group: "review", count: 11 },
      { label: "Preview", group: "review" },
      { label: "Save", group: "edit" },
    ],
    proposedToolbar: [
      { label: "Sanitize + Review", group: "edit", active: true, count: 7 },
      { label: "Apply suggestions", group: "review" },
      { label: "Manual fix", group: "review" },
      { label: "Save", group: "edit", count: 11 },
    ],
    currentFlow: [
      "Run sanitize",
      "Open Epic Conflicts separately",
      "Open Details separately",
      "Save",
    ],
    proposedFlow: [
      "Run sanitize + auto-open review panel",
      "Resolve conflicts and preview in one place",
      "Save from same context",
    ],
  },
];

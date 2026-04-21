<script lang="ts">
  import WorkItemStatistics from "../../components/WorkItemStatistics.svelte";
  import type { Data } from "$lib/bindings/Data";
  import type { Fields } from "$lib/bindings/Fields";
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import type { WorkItemContext } from "$lib/WorkItemContext.svelte";

  const mockData: Data = {
    fields: {
      projectId: "p1",
      status: {
        id: "status",
        name: "Status",
        options: [
          { id: "s-planning", value: "Planning", data: null },
          { id: "s-active", value: "Active", data: null },
        ],
      },
      blocked: { id: "blocked", name: "Blocked", options: [] },
      epic: {
        id: "epic",
        name: "Epic",
        options: [
          { id: "e-platform", value: "Platform", data: null },
          { id: "e-search", value: "Search", data: null },
        ],
      },
      iteration: { id: "iteration", name: "Iteration", options: [] },
      kind: {
        id: "kind",
        name: "Kind",
        options: [
          { id: "k-bug", value: "Bug", data: null },
          { id: "k-feature", value: "Feature", data: null },
        ],
      },
      workstream: {
        id: "workstream",
        name: "Workstream",
        options: [
          { id: "w-ui", value: "UI", data: null },
          { id: "w-api", value: "API", data: null },
        ],
      },
      estimate: { id: "estimate", name: "Estimate", options: [] },
      priority: { id: "priority", name: "Priority", options: [] },
    },
    workItems: {
      "i-1": {
        id: "i-1",
        title: "Bug 1",
        updatedAt: "2026-01-01T00:00:00Z",
        resourcePath: null,
        repoNameWithOwner: null,
        data: {
          type: "issue",
          parentId: null,
          issueType: { loadState: "loaded", value: null },
          state: { loadState: "loaded", value: "OPEN" },
          subIssues: [],
          trackedIssues: { loadState: "loaded", value: [] },
          assignees: ["alice"],
        },
        projectItem: {
          id: "p-1",
          databaseId: null,
          updatedAt: "2026-01-01T00:00:00Z",
          status: "s-active",
          iteration: { loadState: "loaded", value: null },
          blocked: { loadState: "loaded", value: null },
          kind: { loadState: "loaded", value: "k-bug" },
          epic: "e-platform",
          workstream: { loadState: "loaded", value: "w-api" },
          estimate: null,
          priority: null,
        },
      },
      "i-2": {
        id: "i-2",
        title: "Bug 2",
        updatedAt: "2026-01-01T00:00:00Z",
        resourcePath: null,
        repoNameWithOwner: null,
        data: {
          type: "issue",
          parentId: null,
          issueType: { loadState: "loaded", value: null },
          state: { loadState: "loaded", value: "OPEN" },
          subIssues: [],
          trackedIssues: { loadState: "loaded", value: [] },
          assignees: [],
        },
        projectItem: {
          id: "p-2",
          databaseId: null,
          updatedAt: "2026-01-01T00:00:00Z",
          status: "s-planning",
          iteration: { loadState: "loaded", value: null },
          blocked: { loadState: "loaded", value: null },
          kind: { loadState: "loaded", value: "k-bug" },
          epic: "e-search",
          workstream: { loadState: "loaded", value: "w-ui" },
          estimate: null,
          priority: null,
        },
      },
      "i-3": {
        id: "i-3",
        title: "Feature 1",
        updatedAt: "2026-01-01T00:00:00Z",
        resourcePath: null,
        repoNameWithOwner: null,
        data: {
          type: "issue",
          parentId: null,
          issueType: { loadState: "loaded", value: null },
          state: { loadState: "loaded", value: "OPEN" },
          subIssues: [],
          trackedIssues: { loadState: "loaded", value: [] },
          assignees: ["bob"],
        },
        projectItem: {
          id: "p-3",
          databaseId: null,
          updatedAt: "2026-01-01T00:00:00Z",
          status: "s-active",
          iteration: { loadState: "loaded", value: null },
          blocked: { loadState: "loaded", value: null },
          kind: { loadState: "loaded", value: "k-feature" },
          epic: "e-platform",
          workstream: { loadState: "loaded", value: "w-ui" },
          estimate: null,
          priority: null,
        },
      },
      "i-4": {
        id: "i-4",
        title: "Feature 2",
        updatedAt: "2026-01-01T00:00:00Z",
        resourcePath: null,
        repoNameWithOwner: null,
        data: {
          type: "issue",
          parentId: null,
          issueType: { loadState: "loaded", value: null },
          state: { loadState: "loaded", value: "OPEN" },
          subIssues: [],
          trackedIssues: { loadState: "loaded", value: [] },
          assignees: ["alice", "bob"],
        },
        projectItem: {
          id: "p-4",
          databaseId: null,
          updatedAt: "2026-01-01T00:00:00Z",
          status: "s-planning",
          iteration: { loadState: "loaded", value: null },
          blocked: { loadState: "loaded", value: null },
          kind: { loadState: "loaded", value: "k-feature" },
          epic: "e-search",
          workstream: { loadState: "loaded", value: "w-api" },
          estimate: null,
          priority: null,
        },
      },
      "i-5": {
        id: "i-5",
        title: "Bug 3",
        updatedAt: "2026-01-01T00:00:00Z",
        resourcePath: null,
        repoNameWithOwner: null,
        data: {
          type: "issue",
          parentId: null,
          issueType: { loadState: "loaded", value: null },
          state: { loadState: "loaded", value: "OPEN" },
          subIssues: [],
          trackedIssues: { loadState: "loaded", value: [] },
          assignees: ["carol"],
        },
        projectItem: {
          id: "p-5",
          databaseId: null,
          updatedAt: "2026-01-01T00:00:00Z",
          status: "s-active",
          iteration: { loadState: "loaded", value: null },
          blocked: { loadState: "loaded", value: null },
          kind: { loadState: "loaded", value: "k-bug" },
          epic: "e-platform",
          workstream: { loadState: "loaded", value: "w-api" },
          estimate: null,
          priority: null,
        },
      },
    },
    nodes: [
      {
        level: 0,
        id: "i-1",
        hasChildren: false,
        isModified: false,
        data: { type: "workItem" },
      },
      {
        level: 0,
        id: "i-2",
        hasChildren: false,
        isModified: false,
        data: { type: "workItem" },
      },
      {
        level: 0,
        id: "i-3",
        hasChildren: false,
        isModified: false,
        data: { type: "workItem" },
      },
      {
        level: 0,
        id: "i-4",
        hasChildren: false,
        isModified: false,
        data: { type: "workItem" },
      },
      {
        level: 0,
        id: "i-5",
        hasChildren: false,
        isModified: false,
        data: { type: "workItem" },
      },
    ],
    originalWorkItems: {},
    filters: {
      status: [],
      blocked: [],
      epic: [],
      iteration: [],
      kind: [],
      workstream: [],
      estimate: [],
      priority: [],
    },
    changes: { data: {} },
    canUndo: false,
    canRedo: false,
    epicConflicts: [],
  };

  const mockContext: Pick<
    WorkItemContext,
    "data" | "getFieldOption" | "updateWorkItem" | "loadProgress"
  > = {
    data: mockData,
    loadProgress: 0,
    async updateWorkItem() {},
    getFieldOption(fieldName: keyof Fields, id: FieldOptionId | null) {
      if (!id) return undefined;
      const field = this.data.fields[fieldName];
      if (typeof field === "string") return undefined;
      return field.options.find((o) => o.id === id)?.value;
    },
  };
</script>

<div class="h-screen">
  <WorkItemStatistics context={mockContext} />
</div>

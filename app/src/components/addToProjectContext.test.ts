import { describe, expect, it } from "vitest";
import type { WorkItem } from "$lib/bindings/WorkItem";
import type { WorkItemId } from "$lib/bindings/WorkItemId";
import { findAddToProjectContext } from "./addToProjectContext";

function makeIssue(opts: {
  id: string;
  title?: string;
  parentId?: string | null;
  subIssues?: string[];
}): WorkItem {
  return {
    id: opts.id as WorkItemId,
    title: opts.title ?? `Item ${opts.id}`,
    updatedAt: "2024-01-01T00:00:00Z",
    resourcePath: `/owner/repo/issues/${opts.id}`,
    repoNameWithOwner: "owner/repo",
    data: {
      type: "issue",
      parentId: (opts.parentId ?? null) as WorkItemId | null,
      issueType: { loadState: "loaded", value: null },
      state: { loadState: "loaded", value: "OPEN" },
      subIssues: (opts.subIssues ?? []) as WorkItemId[],
      trackedIssues: { loadState: "loaded", value: [] },
      assignees: [],
    },
    projectItem: {
      id: `pi-${opts.id}`,
      databaseId: null,
      updatedAt: "2024-01-01T00:00:00Z",
      status: null,
      iteration: { loadState: "loaded", value: null },
      blocked: { loadState: "loaded", value: null },
      kind: { loadState: "loaded", value: null },
      epic: null,
      workstream: { loadState: "loaded", value: null },
      estimate: null,
      priority: null,
    },
  } as unknown as WorkItem;
}

function makeMap(items: WorkItem[]): { [key in WorkItemId]?: WorkItem } {
  const out: { [key in WorkItemId]?: WorkItem } = {};
  for (const item of items) out[item.id] = item;
  return out;
}

describe("findAddToProjectContext", () => {
  it("returns null when nothing references the id", () => {
    const map = makeMap([makeIssue({ id: "a" })]);
    expect(findAddToProjectContext("missing" as WorkItemId, map)).toBeNull();
  });

  it("finds a 'parent' relationship when a known item points to the id as its parent", () => {
    const child = makeIssue({ id: "child", parentId: "missing-parent" });
    const map = makeMap([child]);
    const ctx = findAddToProjectContext(
      "missing-parent" as WorkItemId,
      map
    );
    expect(ctx).not.toBeNull();
    expect(ctx?.kind).toBe("parent");
    expect(ctx?.referrer.id).toBe("child");
  });

  it("finds a 'subIssue' relationship when a known item lists the id in subIssues", () => {
    const parent = makeIssue({ id: "parent", subIssues: ["missing-child"] });
    const map = makeMap([parent]);
    const ctx = findAddToProjectContext(
      "missing-child" as WorkItemId,
      map
    );
    expect(ctx).not.toBeNull();
    expect(ctx?.kind).toBe("subIssue");
    expect(ctx?.referrer.id).toBe("parent");
  });

  it("prefers a 'parent' match over a 'subIssue' match when both exist", () => {
    const child = makeIssue({ id: "child", parentId: "missing" });
    const sibling = makeIssue({ id: "sibling", subIssues: ["missing"] });
    const map = makeMap([sibling, child]);
    const ctx = findAddToProjectContext("missing" as WorkItemId, map);
    expect(ctx?.kind).toBe("parent");
    expect(ctx?.referrer.id).toBe("child");
  });

  it("ignores non-issue items", () => {
    // A pull request work item has data.type === "pullRequest" and no
    // parentId / subIssues fields — make sure we don't crash.
    const pr = {
      id: "pr1" as WorkItemId,
      title: "A PR",
      updatedAt: "2024-01-01T00:00:00Z",
      resourcePath: "/owner/repo/pull/1",
      repoNameWithOwner: "owner/repo",
      data: {
        type: "pullRequest",
        state: { loadState: "loaded", value: "OPEN" },
        assignees: [],
      },
      projectItem: {
        id: "pi-pr1",
        databaseId: null,
        updatedAt: "2024-01-01T00:00:00Z",
        status: null,
        iteration: { loadState: "loaded", value: null },
        blocked: { loadState: "loaded", value: null },
        kind: { loadState: "loaded", value: null },
        epic: null,
        workstream: { loadState: "loaded", value: null },
        estimate: null,
        priority: null,
      },
    } as unknown as WorkItem;
    const map = makeMap([pr]);
    expect(findAddToProjectContext("anything" as WorkItemId, map)).toBeNull();
  });
});

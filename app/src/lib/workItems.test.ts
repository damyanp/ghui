import { describe, expect, it } from "vitest";
import type { WorkItem } from "./bindings/WorkItem";
import { upsertWorkItem } from "./workItems";

function makeWorkItem(id: string, title: string): WorkItem {
  return {
    id,
    title,
    updatedAt: "2026-01-01T00:00:00Z",
    projectItem: {
      id: `project-${id}`,
      databaseId: "1",
      updatedAt: "2026-01-01T00:00:00Z",
      status: null,
      iteration: { loadState: "loaded", value: null },
      blocked: { loadState: "loaded", value: null },
      kind: { loadState: "loaded", value: null },
      epic: null,
      workstream: { loadState: "loaded", value: null },
      estimate: null,
      priority: null,
    },
    data: { type: "draftIssue" },
    resourcePath: null,
    repoNameWithOwner: null,
  };
}

describe("upsertWorkItem", () => {
  it("returns a new map with the updated item", () => {
    const oldItem = makeWorkItem("WI_1", "Old title");
    const newItem = makeWorkItem("WI_1", "New title");
    const workItems = { [oldItem.id]: oldItem };

    const updated = upsertWorkItem(workItems, newItem);

    expect(updated).not.toBe(workItems);
    expect(updated[oldItem.id]).toEqual(newItem);
  });
});

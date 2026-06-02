import { describe, expect, it } from "vitest";
import type { Data } from "./bindings/Data";
import type { Field } from "./bindings/Field";
import type { Iteration } from "./bindings/Iteration";
import type { SingleSelect } from "./bindings/SingleSelect";
import type { WorkItem } from "./bindings/WorkItem";
import {
  getAllAssignees,
  getAssignees,
  getFilterableFieldOptionIds,
  getFilterableFieldValue,
  getFilterableFields,
  isFilterableField,
} from "./filterableFields";

function singleSelect(...ids: string[]): Field<SingleSelect> {
  return {
    id: "f",
    name: "f",
    options: ids.map((id) => ({ id, value: id, data: null })),
  };
}

function iteration(...ids: string[]): Field<Iteration> {
  return {
    id: "f",
    name: "f",
    options: ids.map((id) => ({
      id,
      value: id,
      data: { startDate: "2024-01-01", duration: 7n },
    })),
  };
}

function makeData(): Data {
  return {
    fields: {
      projectId: "p",
      status: singleSelect("s1", "s2"),
      blocked: singleSelect("b1"),
      epic: singleSelect("e1", "e2"),
      iteration: iteration("i1", "i2"),
      kind: singleSelect("k1"),
      workstream: singleSelect("w1"),
      estimate: singleSelect("est1"),
      priority: singleSelect("p1"),
    },
    workItems: {},
    nodes: [],
    filters: {
      status: [],
      blocked: [],
      epic: [],
      iteration: [],
      kind: [],
      workstream: [],
      estimate: [],
      priority: [],
      assignee: [],
      hideClosed: false,
    },
  } as unknown as Data;
}

function makeWorkItem(overrides: Partial<WorkItem["projectItem"]>): WorkItem {
  return {
    projectItem: {
      id: "pi",
      databaseId: null,
      updatedAt: "",
      status: null,
      iteration: { loadState: "notLoaded" },
      blocked: { loadState: "notLoaded" },
      kind: { loadState: "notLoaded" },
      epic: null,
      workstream: { loadState: "notLoaded" },
      estimate: null,
      priority: null,
      ...overrides,
    },
  } as unknown as WorkItem;
}

describe("filterable field metadata", () => {
  it("getFilterableFields returns the eight known filterable fields", () => {
    expect(getFilterableFields(makeData()).sort()).toEqual(
      [
        "blocked",
        "epic",
        "estimate",
        "iteration",
        "kind",
        "priority",
        "status",
        "workstream",
      ].sort()
    );
  });

  it("isFilterableField is true for each filterable field and false otherwise", () => {
    const data = makeData();
    for (const f of [
      "status",
      "blocked",
      "epic",
      "iteration",
      "kind",
      "workstream",
      "estimate",
      "priority",
    ]) {
      expect(isFilterableField(data, f)).toBe(true);
    }
    expect(isFilterableField(data, "title")).toBe(false);
    expect(isFilterableField(data, "assignees")).toBe(false);
    expect(isFilterableField(data, "")).toBe(false);
    // Inherited Object.prototype property names must not be classified as
    // filterable — guards against the `in` operator pitfall.
    expect(isFilterableField(data, "__proto__")).toBe(false);
    expect(isFilterableField(data, "toString")).toBe(false);
    expect(isFilterableField(data, "hasOwnProperty")).toBe(false);
    // `hideClosed` lives on Filters but is a render-only boolean toggle, not
    // a per-field option-id list — it must NOT be reported as filterable.
    expect(isFilterableField(data, "hideClosed")).toBe(false);
    // `assignee` is a free-form login list, not an option-id field, so it is
    // handled by a dedicated menu and must NOT be reported as filterable.
    expect(isFilterableField(data, "assignee")).toBe(false);
  });

  it("getFilterableFields excludes non-field filter keys like hideClosed", () => {
    expect(getFilterableFields(makeData())).not.toContain("hideClosed");
    expect(getFilterableFields(makeData())).not.toContain("assignee");
  });
});

describe("getAssignees / getAllAssignees", () => {
  function workItemWithData(data: unknown): WorkItem {
    return { data } as unknown as WorkItem;
  }

  it("getAssignees returns assignees for issues and pull requests", () => {
    expect(
      getAssignees(workItemWithData({ type: "issue", assignees: ["alice"] }))
    ).toEqual(["alice"]);
    expect(
      getAssignees(
        workItemWithData({ type: "pullRequest", assignees: ["bob", "carol"] })
      )
    ).toEqual(["bob", "carol"]);
  });

  it("getAssignees returns [] for draft issues", () => {
    expect(getAssignees(workItemWithData({ type: "draftIssue" }))).toEqual([]);
  });

  it("getAllAssignees returns the sorted, de-duplicated set across items", () => {
    const data = {
      ...makeData(),
      workItems: {
        a: workItemWithData({ type: "issue", assignees: ["bob", "alice"] }),
        b: workItemWithData({ type: "pullRequest", assignees: ["alice"] }),
        c: workItemWithData({ type: "draftIssue" }),
      },
    } as unknown as Data;
    expect(getAllAssignees(data)).toEqual(["alice", "bob"]);
  });
});

describe("getFilterableFieldValue", () => {
  it("returns raw FieldOptionId for unwrapped fields (status, epic, estimate, priority)", () => {
    const item = makeWorkItem({
      status: "s1",
      epic: "e1",
      estimate: "est1",
      priority: "p1",
    });
    expect(getFilterableFieldValue(item, "status")).toBe("s1");
    expect(getFilterableFieldValue(item, "epic")).toBe("e1");
    expect(getFilterableFieldValue(item, "estimate")).toBe("est1");
    expect(getFilterableFieldValue(item, "priority")).toBe("p1");
  });

  it("returns null for unset raw fields", () => {
    const item = makeWorkItem({});
    expect(getFilterableFieldValue(item, "status")).toBe(null);
    expect(getFilterableFieldValue(item, "epic")).toBe(null);
    expect(getFilterableFieldValue(item, "estimate")).toBe(null);
    expect(getFilterableFieldValue(item, "priority")).toBe(null);
  });

  it("unwraps loaded DelayLoad fields (iteration, blocked, kind, workstream)", () => {
    const item = makeWorkItem({
      iteration: { loadState: "loaded", value: "i1" },
      blocked: { loadState: "loaded", value: "b1" },
      kind: { loadState: "loaded", value: "k1" },
      workstream: { loadState: "loaded", value: "w1" },
    });
    expect(getFilterableFieldValue(item, "iteration")).toBe("i1");
    expect(getFilterableFieldValue(item, "blocked")).toBe("b1");
    expect(getFilterableFieldValue(item, "kind")).toBe("k1");
    expect(getFilterableFieldValue(item, "workstream")).toBe("w1");
  });

  it("returns loaded null value for DelayLoad fields whose value is null", () => {
    const item = makeWorkItem({
      iteration: { loadState: "loaded", value: null },
    });
    expect(getFilterableFieldValue(item, "iteration")).toBe(null);
  });

  it("returns undefined for not-yet-loaded DelayLoad fields so callers can distinguish unset from unknown", () => {
    const item = makeWorkItem({
      iteration: { loadState: "notLoaded" },
      blocked: { loadState: "notLoaded" },
      kind: { loadState: "notLoaded" },
      workstream: { loadState: "notLoaded" },
    });
    expect(getFilterableFieldValue(item, "iteration")).toBe(undefined);
    expect(getFilterableFieldValue(item, "blocked")).toBe(undefined);
    expect(getFilterableFieldValue(item, "kind")).toBe(undefined);
    expect(getFilterableFieldValue(item, "workstream")).toBe(undefined);
  });
});

describe("getFilterableFieldOptionIds", () => {
  it("prepends null for the 'unset' option for single-select fields", () => {
    expect(getFilterableFieldOptionIds(makeData(), "status")).toEqual([
      null,
      "s1",
      "s2",
    ]);
    expect(getFilterableFieldOptionIds(makeData(), "epic")).toEqual([
      null,
      "e1",
      "e2",
    ]);
  });

  it("works uniformly for iteration fields", () => {
    expect(getFilterableFieldOptionIds(makeData(), "iteration")).toEqual([
      null,
      "i1",
      "i2",
    ]);
  });

  it("returns just [null] when a field has no options", () => {
    const data = makeData();
    data.fields.status = singleSelect();
    expect(getFilterableFieldOptionIds(data, "status")).toEqual([null]);
  });
});

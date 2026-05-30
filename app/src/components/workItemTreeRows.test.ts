import { describe, expect, it } from "vitest";
import type { Node } from "$lib/bindings/Node";
import {
  computeGroupChildCounts,
  computeGroupSiblingCounts,
  computeVisibleRows,
} from "./workItemTreeRows";

function group(id: string, level: number): Node {
  return {
    id,
    level,
    data: { type: "group", name: id, fieldOptionId: null },
    hasChildren: true,
    isModified: false,
    isGhost: false,
  };
}

function item(id: string, level: number): Node {
  return {
    id,
    level,
    data: { type: "workItem", workItemId: { id } as never },
    hasChildren: false,
    isModified: false,
    isGhost: false,
  };
}

describe("computeGroupChildCounts", () => {
  it("counts workItem descendants per group", () => {
    const nodes = [
      group("g1", 0),
      item("a", 1),
      item("b", 1),
      group("g2", 0),
      item("c", 1),
    ];
    const counts = computeGroupChildCounts(nodes);
    expect(counts.get("g1")).toBe(2);
    expect(counts.get("g2")).toBe(1);
  });
});

describe("computeGroupSiblingCounts", () => {
  it("counts buckets sharing the same root parent", () => {
    const nodes = [
      group("g1", 0),
      item("a", 1),
      group("g2", 0),
      item("b", 1),
    ];
    const counts = computeGroupSiblingCounts(nodes);
    expect(counts.get("g1")).toBe(2);
    expect(counts.get("g2")).toBe(2);
  });

  it("reports 1 when a bucket is the only value among its siblings", () => {
    const nodes = [group("only", 0), item("a", 1), item("b", 1)];
    const counts = computeGroupSiblingCounts(nodes);
    expect(counts.get("only")).toBe(1);
  });

  it("scopes siblings to their nearest enclosing parent", () => {
    // g1 has two child buckets (gA, gB); g2 has a single child bucket (gC).
    const nodes = [
      group("g1", 0),
      group("gA", 1),
      item("a", 2),
      group("gB", 1),
      item("b", 2),
      group("g2", 0),
      group("gC", 1),
      item("c", 2),
    ];
    const counts = computeGroupSiblingCounts(nodes);
    expect(counts.get("g1")).toBe(2); // g1 and g2 are top-level siblings
    expect(counts.get("g2")).toBe(2);
    expect(counts.get("gA")).toBe(2); // gA and gB share parent g1
    expect(counts.get("gB")).toBe(2);
    expect(counts.get("gC")).toBe(1); // gC is the only bucket under g2
  });
});

describe("computeVisibleRows", () => {
  const nodes = [group("only", 0), item("a", 1), item("b", 1)];

  it("keeps every row when the toggle is off", () => {
    const rows = computeVisibleRows(nodes, false);
    expect(rows.map((r) => r.id)).toEqual(["only", "a", "b"]);
    expect(rows.map((r) => r.level)).toEqual([0, 1, 1]);
  });

  it("collapses a single-distinct-value bucket and promotes its items", () => {
    const rows = computeVisibleRows(nodes, true);
    expect(rows.map((r) => r.id)).toEqual(["a", "b"]);
    // The lone bucket is removed, so its items rise to the bucket's level.
    expect(rows.map((r) => r.level)).toEqual([0, 0]);
  });

  it("still collapses single-item buckets", () => {
    const single = [
      group("g1", 0),
      item("a", 1),
      group("g2", 0),
      item("b", 1),
      item("c", 1),
    ];
    const rows = computeVisibleRows(single, true);
    // g1 has one item -> collapsed; g2 has two items but is one of two
    // sibling buckets -> kept.
    expect(rows.map((r) => r.id)).toEqual(["a", "g2", "b", "c"]);
    expect(rows.map((r) => r.level)).toEqual([0, 0, 1, 1]);
  });

  it("promotes a collapsed bucket's items under their work-item parent", () => {
    // Reproduces the reported regression: a parent work item whose only child
    // bucket is a single-distinct-value group. Collapsing the group must keep
    // the grandchild visible as a direct child of the parent (level 1), not
    // drop it and leave the parent with a dangling expander.
    const tree = [
      item("parent", 0),
      group("bucket", 1),
      item("child", 2),
    ];
    const rows = computeVisibleRows(tree, true);
    expect(rows.map((r) => r.id)).toEqual(["parent", "child"]);
    expect(rows.map((r) => r.level)).toEqual([0, 1]);
  });

  it("stacks the shift across nested collapsed buckets", () => {
    const tree = [
      item("parent", 0),
      group("outer", 1),
      group("inner", 2),
      item("child", 3),
    ];
    const rows = computeVisibleRows(tree, true);
    expect(rows.map((r) => r.id)).toEqual(["parent", "child"]);
    expect(rows.map((r) => r.level)).toEqual([0, 1]);
  });

  it("does not shift siblings of a collapsed bucket", () => {
    // Two top-level buckets with two items each are kept (siblingCount 2,
    // childCount 2). Verify their levels remain unchanged.
    const tree = [
      group("g1", 0),
      item("a", 1),
      item("b", 1),
      group("g2", 0),
      item("c", 1),
      item("d", 1),
    ];
    const rows = computeVisibleRows(tree, true);
    expect(rows.map((r) => r.id)).toEqual(["g1", "a", "b", "g2", "c", "d"]);
    expect(rows.map((r) => r.level)).toEqual([0, 1, 1, 0, 1, 1]);
  });

  it("decorates rows with isGroup", () => {
    const rows = computeVisibleRows(nodes, false);
    expect(rows.find((r) => r.id === "only")?.isGroup).toBe(true);
    expect(rows.find((r) => r.id === "a")?.isGroup).toBe(false);
  });
});

import { describe, expect, it, vi } from "vitest";
import {
  findPrimaryRow,
  ghostContextMenuItems,
  isRowDraggable,
  type GhostAwareRow,
} from "./ghostRouting";

type Row = GhostAwareRow;

/** Build a work-item row. `id` is the render-position-unique key (in
 * production, the path-prefixed `Node.id` from the recipe builder) and
 * defaults to a synthesised value so tests that care about uniqueness
 * don't have to invent one for every row. */
function workItemRow(
  workItemId: string,
  opts: Partial<Omit<Row, "data">> & { id?: string } = {},
): Row {
  return {
    id: opts.id ?? `row-${workItemId}-${opts.isGhost ? "ghost" : "primary"}`,
    isGhost: false,
    isGroup: false,
    data: { type: "workItem", workItemId },
    ...opts,
  };
}

function groupRow(id: string, opts: Partial<Omit<Row, "data">> = {}): Row {
  return {
    id,
    isGhost: false,
    isGroup: true,
    data: { type: "group" },
    ...opts,
  };
}

describe("findPrimaryRow", () => {
  it("returns the non-ghost row whose data.workItemId matches", () => {
    // Ghosts and primaries share the same workItemId but have distinct
    // render-ids (because Node.id is path-prefixed). The helper must
    // pick the non-ghost occurrence even when the ghost appears first
    // in the list.
    const rows: Row[] = [
      workItemRow("a"),
      workItemRow("b", { isGhost: true }),
      workItemRow("b"),
      workItemRow("c"),
    ];
    expect(findPrimaryRow(rows, "b")).toEqual(
      rows.find((r) => !r.isGhost && r.data.type === "workItem" && r.data.workItemId === "b"),
    );
  });

  it("ignores ghost rows even when their workItemId matches", () => {
    const rows: Row[] = [workItemRow("a"), workItemRow("b", { isGhost: true })];
    expect(findPrimaryRow(rows, "b")).toBeUndefined();
  });

  it("returns undefined when no row matches the workItemId", () => {
    const rows: Row[] = [workItemRow("a"), workItemRow("b")];
    expect(findPrimaryRow(rows, "missing")).toBeUndefined();
  });

  it("skips group rows even when their render-id happens to collide", () => {
    // Group rows never carry a workItemId, so they should never be
    // returned as a "primary" — even if a caller accidentally passes
    // a string that matches a group's render-id.
    const rows: Row[] = [groupRow("b"), workItemRow("b")];
    const primary = findPrimaryRow(rows, "b");
    expect(primary).toBeDefined();
    expect(primary?.data.type).toBe("workItem");
  });
});

describe("isRowDraggable", () => {
  it("is true for a plain non-ghost, non-group row", () => {
    expect(isRowDraggable(workItemRow("a"))).toBe(true);
  });

  it("is false for a ghost row", () => {
    expect(isRowDraggable(workItemRow("a", { isGhost: true }))).toBe(false);
  });

  it("is false for a group row", () => {
    expect(isRowDraggable(groupRow("a"))).toBe(false);
  });

  it("is false for a row that is both ghost and group", () => {
    expect(
      isRowDraggable(groupRow("a", { isGhost: true })),
    ).toBe(false);
  });
});

describe("ghostContextMenuItems", () => {
  it("returns a single 'Jump to primary occurrence' action when a primary exists", () => {
    const primary = workItemRow("b", { id: "primary-row" });
    const ghost = workItemRow("b", { id: "ghost-row", isGhost: true });
    const rows: Row[] = [workItemRow("a"), primary, ghost];
    const jumpTo = vi.fn();

    const items = ghostContextMenuItems(rows, "b", jumpTo);

    expect(items).toHaveLength(1);
    const item = items[0];
    expect(item.type).toBe("action");
    if (item.type !== "action") throw new Error("expected action item");
    expect(item.title).toBe("Jump to primary occurrence");

    item.action();
    expect(jumpTo).toHaveBeenCalledTimes(1);
    // jumpTo receives the primary's render-id, not the workItemId — so the
    // caller can scroll the right DOM row into view via `data-row-id`.
    expect(jumpTo).toHaveBeenCalledWith("primary-row");
  });

  it("returns a text-only fallback when the ghost has no primary in view", () => {
    const rows: Row[] = [workItemRow("b", { isGhost: true })];
    const jumpTo = vi.fn();

    const items = ghostContextMenuItems(rows, "b", jumpTo);

    expect(items).toHaveLength(1);
    expect(items[0].type).toBe("text");
    expect(jumpTo).not.toHaveBeenCalled();
  });

  it("never returns an empty list (menu must not render blank)", () => {
    expect(ghostContextMenuItems([], "missing", vi.fn())).not.toHaveLength(0);
  });
});


import { describe, expect, it, vi } from "vitest";
import {
  findPrimaryRow,
  ghostContextMenuItems,
  isRowDraggable,
  type GhostAwareRow,
} from "./ghostRouting";

type Row = GhostAwareRow;

function row(id: string, opts: Partial<Row> = {}): Row {
  return { id, isGhost: false, isGroup: false, ...opts };
}

describe("findPrimaryRow", () => {
  it("returns the non-ghost row matching `id`", () => {
    // Ghosts and primaries share the same id (both refer to the same
    // WorkItemId); the helper must pick the non-ghost occurrence even when
    // the ghost appears first in the list.
    const rows: Row[] = [
      row("a"),
      row("b", { isGhost: true }),
      row("b"),
      row("c"),
    ];
    expect(findPrimaryRow(rows, "b")).toEqual(row("b"));
  });

  it("ignores ghost rows even when their id matches", () => {
    const rows: Row[] = [row("a"), row("b", { isGhost: true })];
    expect(findPrimaryRow(rows, "b")).toBeUndefined();
  });

  it("returns undefined when no row matches the id", () => {
    const rows: Row[] = [row("a"), row("b")];
    expect(findPrimaryRow(rows, "missing")).toBeUndefined();
  });
});

describe("isRowDraggable", () => {
  it("is true for a plain non-ghost, non-group row", () => {
    expect(isRowDraggable(row("a"))).toBe(true);
  });

  it("is false for a ghost row", () => {
    expect(isRowDraggable(row("a", { isGhost: true }))).toBe(false);
  });

  it("is false for a group row", () => {
    expect(isRowDraggable(row("a", { isGroup: true }))).toBe(false);
  });

  it("is false for a row that is both ghost and group", () => {
    expect(
      isRowDraggable(row("a", { isGhost: true, isGroup: true })),
    ).toBe(false);
  });
});

describe("ghostContextMenuItems", () => {
  it("returns a single 'Jump to primary occurrence' action when a primary exists", () => {
    const rows: Row[] = [row("a"), row("b"), row("b", { isGhost: true })];
    const jumpTo = vi.fn();

    const items = ghostContextMenuItems(rows, "b", jumpTo);

    expect(items).toHaveLength(1);
    const item = items[0];
    expect(item.type).toBe("action");
    if (item.type !== "action") throw new Error("expected action item");
    expect(item.title).toBe("Jump to primary occurrence");

    item.action();
    expect(jumpTo).toHaveBeenCalledTimes(1);
    expect(jumpTo).toHaveBeenCalledWith("b");
  });

  it("returns a text-only fallback when the ghost has no primary in view", () => {
    const rows: Row[] = [row("b", { isGhost: true })];
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


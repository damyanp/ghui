// Pure helpers for ghost-row routing in the tree table. Kept dependency-free
// so the Svelte component can delegate to them and they can be unit tested
// without the Tauri/Svelte runtime.
//
// A "ghost" row is a reflection of a primary work-item row that has been
// duplicated into a different position in the tree by the pivoting recipe
// builder (e.g. to anchor a child under a status group while its real parent
// lives elsewhere). Ghost rows must look muted, must not be draggable, must
// not show edit/refresh actions, and clicking them should route the user to
// the primary occurrence of the same id.

/** Minimal shape a row needs to expose for ghost-routing logic to work. The
 * generic helpers in this file accept any row that carries an `id` and an
 * `isGhost` flag; `isGroup` is optional because not every caller exposes it.
 */
export type GhostAwareRow = {
  id: string;
  isGhost: boolean;
  isGroup?: boolean;
};

/** Returns the primary (non-ghost) row whose id matches `id`, or `undefined`
 * when no such row exists in `rows`. Ghost rows are never returned, even when
 * their id matches — by definition a ghost is not its own primary. */
export function findPrimaryRow<T extends GhostAwareRow>(
  rows: readonly T[],
  id: string,
): T | undefined {
  return rows.find((r) => r.id === id && !r.isGhost);
}

/** Whether a row should be user-draggable. Ghost rows are reflections and
 * cannot be re-parented; group rows are structural and aren't draggable
 * either. */
export function isRowDraggable(row: GhostAwareRow): boolean {
  return !row.isGhost && !row.isGroup;
}

/** Subset of {@link MenuItem} (from `TreeTableContextMenu.svelte`) that the
 * ghost context-menu builder produces. Declared locally so this module stays
 * dependency-free; the returned value is assignable to `MenuItem[]`. */
export type GhostMenuItem =
  | { type: "action"; title: string; action: () => void }
  | { type: "text"; title: string };

/** Builds the restricted context-menu shown for ghost rows. When a primary
 * occurrence is present in `rows`, a single "Jump to primary occurrence"
 * action is returned that delegates to the supplied {@link jumpTo} callback.
 * Otherwise a static text item explains why no action is available. The
 * returned list is always non-empty so the menu never renders blank. */
export function ghostContextMenuItems<T extends GhostAwareRow>(
  rows: readonly T[],
  ghostId: string,
  jumpTo: (primaryId: string) => void,
): GhostMenuItem[] {
  const primary = findPrimaryRow(rows, ghostId);
  if (primary) {
    return [
      {
        type: "action",
        title: "Jump to primary occurrence",
        action: () => jumpTo(primary.id),
      },
    ];
  }
  return [
    {
      type: "text",
      title: "Ghost row — primary occurrence not in current view",
    },
  ];
}

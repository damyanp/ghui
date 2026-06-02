import type { Node } from "$lib/bindings/Node";

/** A tree node decorated with whether it is a group (bucket) header row. */
export type TreeRow = Node & { isGroup: boolean };

const ROOT_PARENT_KEY = "\0root";

/**
 * Count of `workItem` descendants for each group node. Computed in a single
 * pass: a stack tracks all open group ancestors; each `workItem` node
 * increments every ancestor's count.
 */
export function computeGroupChildCounts(nodes: Node[]): Map<string, number> {
  const counts = new Map<string, number>();
  const openGroups: { id: string; level: number }[] = [];

  for (const node of nodes) {
    while (
      openGroups.length > 0 &&
      openGroups[openGroups.length - 1].level >= node.level
    ) {
      openGroups.pop();
    }
    if (node.data.type === "group") {
      counts.set(node.id, 0);
      openGroups.push({ id: node.id, level: node.level });
    } else if (node.data.type === "workItem") {
      for (const g of openGroups) {
        counts.set(g.id, (counts.get(g.id) ?? 0) + 1);
      }
    }
  }

  return counts;
}

/**
 * For each group node, the number of sibling buckets that share its parent —
 * i.e. group nodes whose nearest enclosing ancestor (the nearest preceding node
 * at a strictly smaller level) is the same. A count of `1` means the bucket is
 * the only distinct value among its siblings.
 */
export function computeGroupSiblingCounts(nodes: Node[]): Map<string, number> {
  const counts = new Map<string, number>();
  const siblingsByParent = new Map<string, string[]>();
  const ancestors: { id: string; level: number }[] = [];

  for (const node of nodes) {
    while (
      ancestors.length > 0 &&
      ancestors[ancestors.length - 1].level >= node.level
    ) {
      ancestors.pop();
    }
    const parentKey =
      ancestors.length > 0 ? ancestors[ancestors.length - 1].id : ROOT_PARENT_KEY;
    if (node.data.type === "group") {
      const siblings = siblingsByParent.get(parentKey) ?? [];
      siblings.push(node.id);
      siblingsByParent.set(parentKey, siblings);
    }
    ancestors.push({ id: node.id, level: node.level });
  }

  for (const siblings of siblingsByParent.values()) {
    for (const id of siblings) counts.set(id, siblings.length);
  }
  return counts;
}

/**
 * The set of group node ids that are "root" groups — top-level buckets whose
 * nearest enclosing parent is the root (they have no group or work-item
 * ancestor). Root groups are never collapsed by `collapseSingleValue`.
 */
export function computeRootGroupIds(nodes: Node[]): Set<string> {
  const roots = new Set<string>();
  const ancestors: number[] = [];

  for (const node of nodes) {
    while (ancestors.length > 0 && ancestors[ancestors.length - 1] >= node.level) {
      ancestors.pop();
    }
    if (node.data.type === "group" && ancestors.length === 0) {
      roots.add(node.id);
    }
    ancestors.push(node.level);
  }

  return roots;
}

/**
 * Compute the visible rows for the work item tree, applying the
 * `collapseSingleValue` toggle.
 *
 * When `collapseSingleValue` is on, a non-root group (bucket) header row is
 * hidden when either:
 *  - the bucket contains exactly one work item (a single-item bucket), or
 *  - the bucket is the only bucket among its siblings (a single-distinct-value
 *    bucket — every item under the parent shares the same field value).
 *
 * Root (top-level) groups are never collapsed, even when they contain a single
 * issue or are the only bucket at the top level.
 *
 * Hiding a group header would leave its descendants indented one level too deep
 * (the row array is rendered as a tree by `level`, so a gap makes `TreeTable`
 * skip the now-orphaned children entirely). To keep the tree contiguous, each
 * collapsed group's descendants are shifted up by one level, so they slot in as
 * direct children of the collapsed group's parent.
 */
export function computeVisibleRows(
  nodes: Node[],
  collapseSingleValue: boolean
): TreeRow[] {
  if (!collapseSingleValue) {
    return nodes.map((n) => ({ ...n, isGroup: n.data.type === "group" }));
  }

  const childCounts = computeGroupChildCounts(nodes);
  const siblingCounts = computeGroupSiblingCounts(nodes);
  const rootGroups = computeRootGroupIds(nodes);

  const shouldCollapse = (node: Node): boolean =>
    node.data.type === "group" &&
    !rootGroups.has(node.id) &&
    (childCounts.get(node.id) === 1 || siblingCounts.get(node.id) === 1);

  const rows: TreeRow[] = [];
  // Original levels of collapsed group ancestors still enclosing the current
  // node. Its length is how many levels the current node should shift up by.
  const collapsedAncestorLevels: number[] = [];

  for (const node of nodes) {
    while (
      collapsedAncestorLevels.length > 0 &&
      collapsedAncestorLevels[collapsedAncestorLevels.length - 1] >= node.level
    ) {
      collapsedAncestorLevels.pop();
    }

    if (shouldCollapse(node)) {
      collapsedAncestorLevels.push(node.level);
      continue;
    }

    rows.push({
      ...node,
      level: node.level - collapsedAncestorLevels.length,
      isGroup: node.data.type === "group",
    });
  }

  return rows;
}

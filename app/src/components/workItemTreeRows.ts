import type { Node } from "$lib/bindings/Node";

/** A tree node decorated with whether it is a group (bucket) header row. */
export type TreeRow = Node & { isGroup: boolean };

const ROOT_PARENT_KEY = "\0root";

/**
 * Count of `workItem` descendants for each group node. Computed by walking the
 * flat depth-first node array: every descendant of a group is contiguous and at
 * a strictly greater level until we hit a node at `<= group.level`.
 */
export function computeGroupChildCounts(nodes: Node[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (let i = 0; i < nodes.length; i++) {
    const head = nodes[i];
    if (head.data.type !== "group") continue;
    let count = 0;
    for (let j = i + 1; j < nodes.length; j++) {
      const child = nodes[j];
      if (child.level <= head.level) break;
      if (child.data.type === "workItem") count++;
    }
    counts.set(head.id, count);
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
 * Compute the visible rows for the work item tree, applying the
 * `collapseSingleValue` toggle.
 *
 * When `collapseSingleValue` is on, a group (bucket) header row is hidden when
 * either:
 *  - the bucket contains exactly one work item (a single-item bucket), or
 *  - the bucket is the only bucket among its siblings (a single-distinct-value
 *    bucket — every item under the parent shares the same field value).
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

  const shouldCollapse = (node: Node): boolean =>
    node.data.type === "group" &&
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

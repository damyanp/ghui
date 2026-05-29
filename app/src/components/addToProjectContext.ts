import type { WorkItem } from "$lib/bindings/WorkItem";
import type { WorkItemId } from "$lib/bindings/WorkItemId";

/**
 * Describes how an unknown `WorkItemId` (one that's not yet in the project,
 * and therefore not in `workItems`) is referenced by an item that *is* in
 * `workItems`. This is used to give the user meaningful context for
 * `AddToProject` changes — without it the UI can only show the opaque
 * GitHub node ID.
 *
 * - `"parent"` means some known item declares this id as its `parentId`,
 *   i.e. the unknown id will be added to the project as that item's parent.
 * - `"subIssue"` means some known item lists this id in its `subIssues`,
 *   i.e. the unknown id will be added to the project as a sub-issue of
 *   that item.
 */
export type AddToProjectContext = {
  kind: "parent" | "subIssue";
  /** The known work item that references the unknown id. */
  referrer: WorkItem;
};

/**
 * Looks for a work item in the given map that references `workItemId` either
 * as its parent or as one of its sub-issues, and returns the first match.
 *
 * Returns `null` if no referring item is found.
 *
 * Parent matches are preferred over sub-issue matches because a `parentId`
 * is unique per item (an item has at most one parent), whereas the same id
 * can appear as a sub-issue of multiple items in rare/inconsistent states.
 */
export function findAddToProjectContext(
  workItemId: WorkItemId,
  workItems: { [key in WorkItemId]?: WorkItem }
): AddToProjectContext | null {
  let subIssueMatch: WorkItem | null = null;

  for (const item of Object.values(workItems)) {
    if (!item || item.data.type !== "issue") continue;
    if (item.data.parentId === workItemId) {
      return { kind: "parent", referrer: item };
    }
    if (!subIssueMatch && item.data.subIssues.includes(workItemId)) {
      subIssueMatch = item;
    }
  }

  if (subIssueMatch) {
    return { kind: "subIssue", referrer: subIssueMatch };
  }
  return null;
}

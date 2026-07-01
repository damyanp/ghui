import type { WorkItem } from "./bindings/WorkItem";
import type { WorkItemId } from "./bindings/WorkItemId";

export type WorkItemsMap = {
  [key in WorkItemId]?: WorkItem;
};

export function upsertWorkItem(
  workItems: WorkItemsMap,
  workItem: WorkItem
): WorkItemsMap {
  return {
    ...workItems,
    [workItem.id]: workItem,
  };
}

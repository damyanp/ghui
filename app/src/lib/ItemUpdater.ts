import type { WorkItemId } from "./bindings/WorkItemId";
import { invoke } from "@tauri-apps/api/core";
import { ItemUpdateQueue } from "./itemUpdateQueue";

export class ItemUpdateBatcher {
  private queue: ItemUpdateQueue<{
    workItemId: WorkItemId;
    force: boolean;
    accountGeneration: number;
  }>;

  constructor(reportError: (error: unknown) => void) {
    this.queue = new ItemUpdateQueue(
      (items) => invoke("update_items", { items }),
      reportError
    );
  }

  public add(
    workItemId: WorkItemId,
    force: boolean,
    accountGeneration: number
  ): void {
    this.queue.add({ workItemId, force, accountGeneration });
  }

  public clear(): void {
    this.queue.clear();
  }
}

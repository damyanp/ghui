import { tick } from "svelte";
import type { WorkItemId } from "./bindings/WorkItemId";
import { invoke } from "@tauri-apps/api/core";

export class ItemUpdateBatcher {
  currentBatch: Set<{ workItemId: WorkItemId; force: boolean }> = new Set();
  submitPromise?: Promise<void> = undefined;

  public add(workItemId: WorkItemId, force: boolean) {
    this.currentBatch.add({ workItemId, force });

    if (this.submitPromise === undefined) {
      this.submitPromise = new Promise((resolve) => setTimeout(resolve));
      this.submitPromise.then(async () => {
        this.submitPromise = undefined;
        await this.submit();
      });
    }
  }

  async submit() {
    let items = Array.from(this.currentBatch.values());
    this.currentBatch.clear();
    await invoke("update_items", { items });
  }
}

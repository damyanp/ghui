import { getContext, setContext } from "svelte";
import type { Data } from "./bindings/Data";
import { Channel, invoke } from "@tauri-apps/api/core";
import type { WorkItemId } from "./bindings/WorkItemId";

const key = Symbol("WorkItemContext");

export function setWorkItemContext(wic: WorkItemContext) {
  setContext(key, wic);
  return wic;
}

export function getWorkItemContext() {
  return getContext(key) as WorkItemContext;
}

export class WorkItemContext {
  data = $state<Data>({
    workItems: {},
    nodes: [],
    originalWorkItems: {},
    changes: { data: {} },
  });

  loadProgress = $state<number>(0);

  public async refresh(forceRefresh: boolean): Promise<void> {
    if (this.loadProgress !== 0) return;

    this.loadProgress = 1;

    type Progress = number[];
    const getDataProgress = new Channel<Progress>();
    getDataProgress.onmessage = (message) => {
      const [retrieved, total] = message;
      if (total === 0) this.loadProgress = 0;
      else this.loadProgress = 1 - retrieved / total;
    };

    this.data = await invoke<Data>("get_data", {
      forceRefresh: forceRefresh,
      progress: getDataProgress,
    });

    this.loadProgress = 0;
  }

  public async convertTrackedIssuesToSubIssue(id: WorkItemId) {
    await invoke("convert_tracked_to_sub_issues", {
      id,
    });
    await this.refresh(false);
  }
}

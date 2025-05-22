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

    const progress = makeProgressChannel(
      (value) => (this.loadProgress = value)
    );

    this.data = await invoke<Data>("get_data", {
      forceRefresh,
      progress,
    });

    this.loadProgress = 0;
  }

  public async convertTrackedIssuesToSubIssue(id: WorkItemId) {
    await invoke("convert_tracked_to_sub_issues", {
      id,
    });
    await this.refresh(false);
  }

  // #region Managing Changes

  previewChanges = $derived(
    Object.keys(this.data.originalWorkItems).length > 0
  );

  public async deleteChanges() {
    await invoke("delete_changes");
    await this.refresh(false);
  }

  public async setPreviewChanges(preview: boolean) {
    await invoke("set_preview_changes", { preview });
    await this.refresh(false);
  }

  public async saveChanges(progress: Channel<Progress>) {
      await invoke("save_changes", { progress });
    await this.refresh(true);
  }

  // #endregion
}

type Progress = number[];

export function makeProgressChannel(
  setter: (value: number) => void
): Channel<Progress> {
  let progress = 0;
  setter(progress);

  const getDataProgress = new Channel<Progress>();
  getDataProgress.onmessage = (message) => {
    const [retrieved, total] = message;
    if (total === 0) progress = 0;
    else progress = 1 - retrieved / total;
    setter(progress);
  };

  return getDataProgress;
}

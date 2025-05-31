import { getContext, setContext, tick } from "svelte";
import type { Data } from "./bindings/Data";
import { Channel, invoke } from "@tauri-apps/api/core";
import type { WorkItemId } from "./bindings/WorkItemId";
import type { Change } from "./bindings/Change";
import type { Filters } from "./bindings/Filters";
import type { Fields } from "./bindings/Fields";
import type { Field } from "./bindings/Field";
import type { FieldOptionId } from "./bindings/FieldOptionId";

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
    fields: make_blank_fields(),
    workItems: {},
    nodes: [],
    filters: { hideClosed: true },
    originalWorkItems: {},
    changes: { data: {} },
  });

  loadProgress = $state<number>(0);

  public async refresh(forceRefresh: boolean): Promise<void> {
    if (this.loadProgress !== 0) return;

    // If we're expecting this to take a long time start the progress spinner
    // immediately
    if (forceRefresh || this.data.nodes.length === 0) this.loadProgress = 1;

    const progress = makeProgressChannel(
      (value) => (this.loadProgress = value)
    );

    this.data = await invoke<Data>("get_data", {
      forceRefresh,
      progress,
    });

    this.loadProgress = 0;
  }

  public async updateWorkItem(workItemId: WorkItemId) {
    throw new Error("Method not implemented.");
  }

  public async convertTrackedIssuesToSubIssue(id: WorkItemId) {
    await invoke("convert_tracked_to_sub_issues", {
      id,
    });
    await this.refresh(false);
  }

  public async sanitize() {
    await invoke("sanitize");
    await this.refresh(false);
  }

  public get hideClosed() {
    return this.data.filters.hideClosed;
  }

  public set hideClosed(value: boolean) {
    console.log(`set hideClosed: ${value}`);
    this.data.filters.hideClosed = value;
    invoke("set_filters", { filters: this.data.filters }).then(() =>
      this.refresh(false)
    );
  }

  public getFieldOption(
    fieldName: keyof Fields,
    id: FieldOptionId | null
  ): string | undefined {
    if (!id) return undefined;

    const field = this.data.fields[fieldName];

    if (typeof field === "string") return undefined;

    return field.options.find((o) => o.id === id)?.value;
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

  public async addChange(change: Change) {
    await invoke("add_change", { change });
    await this.refresh(false);
  }

  public async removeChange(change: Change) {
    await invoke("remove_change", { change });
    await this.refresh(false);
  }

  // #endregion
}

type Progress = number[];

export function makeProgressChannel(
  setter: (value: number) => void
): Channel<Progress> {
  const getDataProgress = new Channel<Progress>();
  getDataProgress.onmessage = (message) => {
    const [retrieved, total] = message;
    const progress = total === 0 ? 0 : 1 - retrieved / total;
    setter(progress);
  };

  return getDataProgress;
}

function make_blank_fields(): Fields {
  function blank(): Field {
    return {
      id: "",
      name: "",
      field_type: "SingleSelect",
      options: [],
    };
  }
  return {
    project_id: "",
    status: blank(),
    blocked: blank(),
    epic: blank(),
    iteration: blank(),
    kind: blank(),
    project_milestone: blank(),
  };
}

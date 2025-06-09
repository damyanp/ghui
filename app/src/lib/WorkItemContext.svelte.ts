import { getContext, setContext, tick } from "svelte";
import type { Data } from "./bindings/Data";
import { Channel, invoke } from "@tauri-apps/api/core";
import type { WorkItemId } from "./bindings/WorkItemId";
import type { Change } from "./bindings/Change";
import type { Fields } from "./bindings/Fields";
import type { Field } from "./bindings/Field";
import type { FieldOptionId } from "./bindings/FieldOptionId";
import { type DataUpdate } from "./bindings/DataUpdate";
import { ItemUpdateBatcher } from "./ItemUpdater";
import type { WorkItem } from "./bindings/WorkItem";

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

  updates_channel = new Channel<DataUpdate>();

  constructor() {
    this.updates_channel.onmessage = (data_update) =>
      this.on_data_update(data_update);
    tick().then(() => invoke("watch_data", { channel: this.updates_channel }));
  }

  on_data_update(dataUpdate: DataUpdate) {
    switch (dataUpdate.type) {
      case "data":
        this.onDataUpdateData(dataUpdate.value);
        break;
      case "progress":
        this.onDataUpdateProgress(dataUpdate.value);
        break;
      case "workItem":
        this.onDataUpdateWorkItem(dataUpdate.value);
    }
  }

  onDataUpdateData(data: Data) {
    this.data = data;
  }

  onDataUpdateWorkItem(workItem: WorkItem) {
    this.data.workItems[workItem.id] = workItem;
  }

  onDataUpdateProgress({ done, total }: { done: number; total: number }) {
    this.loadProgress = total === 0 ? 0 : 1 - done / total;
  }

  public async refresh(): Promise<void> {
    await invoke("force_refresh_data");
  }

  itemUpdateBatcher = new ItemUpdateBatcher();

  public async updateWorkItem(workItemId: WorkItemId) {
    this.itemUpdateBatcher.add(workItemId, false);
  }

  public async convertTrackedIssuesToSubIssue(id: WorkItemId) {
    await invoke("convert_tracked_to_sub_issues", {
      id,
    });
  }

  public async sanitize() {
    await invoke("sanitize");
  }

  public get hideClosed() {
    return this.data.filters.hideClosed;
  }

  public set hideClosed(value: boolean) {
    console.log(`set hideClosed: ${value}`);
    this.data.filters.hideClosed = value;
    invoke("set_filters", { filters: this.data.filters });
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

  public getField(fieldName: keyof Fields): Field {
    const field = this.data.fields[fieldName];
    if (typeof field === "string")
      throw new Error(`'${fieldName} doesn't refer to a custom field`);
    return field;
  }

  public async setFieldValue(item: WorkItem, field: keyof Fields, value: FieldOptionId|undefined) {
    await invoke("set_field_value", { field, value });
  }

  // #region Managing Changes

  previewChanges = $derived(
    Object.keys(this.data.originalWorkItems).length > 0
  );

  public async deleteChanges() {
    await invoke("delete_changes");
  }

  public async setPreviewChanges(preview: boolean) {
    await invoke("set_preview_changes", { preview });
  }

  public async saveChanges(progress: Channel<Progress>) {
    await invoke("save_changes", { progress });
  }

  public async addChange(change: Change) {
    await invoke("add_change", { change });
  }

  public async removeChange(change: Change) {
    await invoke("remove_change", { change });
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

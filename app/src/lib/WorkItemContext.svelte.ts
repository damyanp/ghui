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
import type { Filters } from "./bindings/Filters";
import type { SingleSelect } from "./bindings/SingleSelect";
import type { Iteration } from "./bindings/Iteration";
import type { ProjectItem } from "./bindings/ProjectItem";

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
    filters: { status: [], blocked: [], epic: [], iteration: [], kind: [] },
    originalWorkItems: {},
    changes: { data: {} },
  });

  workItemTreeExpandedItems = $state<string[]>([]);

  loadProgress = $state<number>(0);

  updates_channel = new Channel<DataUpdate>();

  constructor() {
    this.updates_channel.onmessage = (data_update) =>
      this.on_data_update(data_update);
    tick().then(() => invoke("watch_data", { channel: this.updates_channel }));

    this.workItemExtraData = JSON.parse(
      window.localStorage.getItem("workItemExtraData") || "{}"
    );

    $effect(() => {
      window.localStorage.setItem(
        "workItemExtraData",
        JSON.stringify(this.workItemExtraData, undefined, " ")
      );
    });
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

  public getFieldOption(
    fieldName: keyof Fields,
    id: FieldOptionId | null
  ): string | undefined {
    if (!id) return undefined;

    const field = this.data.fields[fieldName];

    if (typeof field === "string") return undefined;

    return field.options.find((o) => o.id === id)?.value;
  }

  public getSingleSelectField(fieldName: keyof Fields): Field<SingleSelect> {
    switch (fieldName) {
      case "blocked":
      case "epic":
      case "kind":
      case "projectMilestone":
      case "status":
        return this.data.fields[fieldName];

      default:
        throw new Error(`${fieldName} is not a single select field`);
    }
  }

  public getIterationField(fieldName: keyof Fields): Field<Iteration> {
    switch (fieldName) {
      case "iteration":
        return this.data.fields[fieldName];

      default:
        throw new Error(`${fieldName} is not an iteration field`);
    }
  }

  public getIterationFieldValue(
    fieldName: keyof Fields,
    workItem: WorkItem
  ): FieldOptionId | null {
    switch (fieldName as keyof ProjectItem) {
      case "iteration":
        const field = workItem.projectItem.iteration;
        if (field.loadState === "loaded") {
          return field.value;
        }
        return null;

      default:
        throw new Error(`${fieldName} is not an iteration field`);
    }
  }

  public getFilter(fieldName: keyof Fields): Array<FieldOptionId | null> {
    return this.data.filters[fieldName as keyof Filters];
  }

  public setFilter(
    fieldName: keyof Fields,
    filter: Array<FieldOptionId | null>
  ): void {
    this.data.filters[fieldName as keyof Filters] = filter;
    invoke("set_filters", { filters: this.data.filters });
  }

  public async setFieldValue(
    item: WorkItem,
    field: keyof Fields,
    value: FieldOptionId | undefined
  ) {
    switch (field) {
      case "blocked":
      case "iteration":
      case "status":
      case "epic":
      case "kind":
        await this.addChange({
          workItemId: item.id,
          data: {
            type: field,
            value: value || null,
          },
        });
        break;

      default:
        throw new Error(`Change not implemented for ${field}`);
    }
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

  workItemExtraData: { [key in WorkItemId]?: any } = $state({});

  public getWorkItemExtraData(id: WorkItemId): any {
    const d = this.workItemExtraData[id];
    if (d) {
      return d;
    }

    return {};
  }

  public setWorkItemExtraData(id: WorkItemId, data: any) {
    this.workItemExtraData[id] = data;
  }
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
  function blank<T>(): Field<T> {
    return {
      id: "",
      name: "",
      options: [],
    };
  }

  return {
    projectId: "",
    status: blank(),
    blocked: blank(),
    epic: blank(),
    iteration: blank(),
    kind: blank(),
    projectMilestone: blank(),
  };
}

export function linkHRef(item: WorkItem): string {
  const databaseId = item.projectItem.databaseId;
  const owner = item.repoNameWithOwner?.split("/")[0];
  const repo = item.repoNameWithOwner?.split("/")[1];
  const number = item.resourcePath?.split("/")[4];

  if (
    item.data.type !== "pullRequest" &&
    databaseId &&
    owner &&
    repo &&
    number
  ) {
    return `https://github.com/orgs/llvm/projects/4?pane=issue&itemId=${databaseId}&issue=${owner}%7C${repo}%7C${number}`;
  }
  return `https://github.com${item.resourcePath}`;
}

export function linkTitle(item: WorkItem): string {
  const path = item.resourcePath?.split("/");
  return `${path?.at(-3)}#${path?.at(-1)}`;
}
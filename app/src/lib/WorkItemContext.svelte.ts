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

/**
 * The subset of `Filters` whose values are arrays of field option IDs.
 * Excludes boolean/scalar filters such as `hideClosed`.
 */
type FieldOptionFilters = {
  [K in keyof Filters as Filters[K] extends Array<FieldOptionId | null>
    ? K
    : never]: Filters[K];
};
import type { SingleSelect } from "./bindings/SingleSelect";
import type { Iteration } from "./bindings/Iteration";
import type { ProjectItem } from "./bindings/ProjectItem";
import type { LogEntry } from "./bindings/LogEntry";
import type { TelemetryEvent } from "./bindings/TelemetryEvent";
import type { ResolvedUrl } from "./bindings/ResolvedUrl";
import type { RefreshSummary } from "./bindings/RefreshSummary";

const key = Symbol("WorkItemContext");

export function setWorkItemContext(wic: WorkItemContext) {
  setContext(key, wic);
  return wic;
}

export function getWorkItemContext() {
  return getContext(key) as WorkItemContext;
}

/** Fire-and-forget telemetry recording via the backend. */
export function recordTelemetry(event: TelemetryEvent): void {
  invoke("record_telemetry", { event }).catch(() => {});
}

export class WorkItemContext {
  data = $state<Data>({
    fields: make_blank_fields(),
    workItems: {},
    nodes: [],
    filters: {
      status: [],
      blocked: [],
      epic: [],
      iteration: [],
      kind: [],
      workstream: [],
      estimate: [],
      priority: [],
      hideClosed: false,
    },
    originalWorkItems: {},
    changes: { data: {} },
    canUndo: false,
    canRedo: false,
    epicConflicts: [],
  });

  workItemTreeExpandedItems = $state<string[]>([]);

  logs = $state<LogEntry[]>([]);
  unreadErrorCount = $state<number>(0);

  loadProgress = $state<number>(0);

  updates_channel = new Channel<DataUpdate>();

  constructor() {
    this.updates_channel.onmessage = (data_update) =>
      this.on_data_update(data_update);
    tick().then(() => invoke("watch_data", { channel: this.updates_channel }));

    let loadedExtraData = false;
    invoke<string>("get_work_items_extra_data")
      .then((value) => {
        this.workItemExtraData = JSON.parse(value);
        loadedExtraData = true;
      })
      .catch(() => (this.workItemExtraData = {}));

    $effect(() => {
      const extraData = JSON.stringify(this.workItemExtraData, undefined, " ");
      if (loadedExtraData) {
        invoke("set_work_items_extra_data", { extraData });
      }
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
        break;
      case "log":
        this.onDataUpdateLog(dataUpdate.value);
        break;
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

  onDataUpdateLog(entry: LogEntry) {
    this.logs.push(entry);
    if (this.logs.length > 1000) {
      const excess = this.logs.length - 1000;
      this.logs.splice(0, excess);
    }
    if (entry.level === "error") {
      this.unreadErrorCount++;
    }
  }

  markErrorsAsRead() {
    this.unreadErrorCount = 0;
  }

  public async refresh(): Promise<RefreshSummary> {
    return await invoke<RefreshSummary>("force_refresh_data");
  }

  itemUpdateBatcher = new ItemUpdateBatcher();

  public async updateWorkItem(workItemId: WorkItemId) {
    this.itemUpdateBatcher.add(workItemId, false);
  }

  /**
   * Triggers the backend to load full field data for every work item that
   * isn't already loaded. Resolves only after every chunk has completed, so
   * callers can use the returned promise to drive a loading indicator.
   */
  public async loadAllWorkItems(): Promise<void> {
    await invoke("load_all_work_items");
  }

  public async convertTrackedIssuesToSubIssue(id: WorkItemId) {
    await invoke("convert_tracked_to_sub_issues", {
      id,
    });
  }

  public async sanitize() {
    await invoke("sanitize");
  }

  public async stageEpicOverrides(itemIds: WorkItemId[]) {
    await invoke("stage_epic_overrides", { itemIds });
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
      case "status":
      case "workstream":
      case "estimate":
      case "priority":
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
    return this.data.filters[
      fieldName as keyof FieldOptionFilters
    ] as Array<FieldOptionId | null>;
  }

  public setFilter(
    fieldName: keyof Fields,
    filter: Array<FieldOptionId | null>
  ): void {
    (this.data.filters as FieldOptionFilters)[
      fieldName as keyof FieldOptionFilters
    ] = filter;
    invoke("set_filters", { filters: this.data.filters });
  }

  public setHideClosed(hideClosed: boolean): void {
    this.data.filters.hideClosed = hideClosed;
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
      case "workstream":
      case "estimate":
      case "priority":
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

  public async addChanges(changes: Change[]) {
    await invoke("add_changes", { changes });
  }

  public async removeChange(change: Change) {
    await invoke("remove_change", { change });
  }

  public async undoChange() {
    await invoke("undo_change");
  }

  public async redoChange() {
    await invoke("redo_change");
  }

  public async resolveUrl(url: string): Promise<ResolvedUrl> {
    return await invoke<ResolvedUrl>("resolve_url", { url });
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
    workstream: blank(),
    estimate: blank(),
    priority: blank(),
  };
}

export function directLinkHRef(item: WorkItem): string {
  return `https://github.com${item.resourcePath}`;
}

export function projectLinkHRef(item: WorkItem): string | undefined {
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
  return undefined;
}

export function linkTitle(item: WorkItem): string {
  const path = item.resourcePath?.split("/");
  return `${path?.at(-3)}#${path?.at(-1)}`;
}

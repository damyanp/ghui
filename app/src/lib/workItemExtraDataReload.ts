import { mergePendingExtraData } from "./workItemExtraData";

export type ExtraDataReloadCompletion =
  | { type: "loaded"; data: Record<string, unknown> }
  | { type: "failed"; error: Error }
  | { type: "superseded" }
  | { type: "ignored" };

export class WorkItemExtraDataReload {
  private nextRequest = 0;
  private activeRequest: number | null = null;
  private accountKey: string | undefined;
  private captureEdits = false;
  private pendingEdits = new Map<string, unknown>();
  private disposed = false;

  start(expectedAccountKey?: string): {
    request: number;
    superseded: boolean;
  } {
    const request = ++this.nextRequest;
    if (this.disposed) return { request, superseded: false };

    const superseded =
      this.accountKey !== undefined &&
      expectedAccountKey !== undefined &&
      expectedAccountKey !== this.accountKey;
    if (superseded) this.pendingEdits.clear();

    if (expectedAccountKey !== undefined) {
      this.accountKey = expectedAccountKey;
    }
    this.captureEdits = true;
    this.activeRequest = request;
    return { request, superseded };
  }

  bindAccount(request: number, accountKey: string): ExtraDataReloadCompletion {
    if (request !== this.activeRequest) return { type: "ignored" };
    if (this.accountKey === undefined) {
      this.accountKey = accountKey;
      return { type: "ignored" };
    }
    if (this.accountKey === accountKey) return { type: "ignored" };

    this.activeRequest = null;
    this.captureEdits = false;
    this.pendingEdits.clear();
    this.accountKey = accountKey;
    return { type: "superseded" };
  }

  recordEdit(id: string, data: unknown, accountKey: string): boolean {
    if (this.disposed || accountKey !== this.accountKey) return false;
    if (this.captureEdits) this.pendingEdits.set(id, data);
    return true;
  }

  dispose(): void {
    this.disposed = true;
    this.activeRequest = null;
    this.captureEdits = false;
    this.pendingEdits.clear();
    this.accountKey = undefined;
  }

  fail(request: number, error: unknown): ExtraDataReloadCompletion {
    if (request !== this.activeRequest) return { type: "ignored" };
    this.activeRequest = null;
    return {
      type: "failed",
      error: error instanceof Error ? error : new Error(String(error)),
    };
  }

  finish(
    request: number,
    actualAccountKey: string,
    serializedData: string
  ): ExtraDataReloadCompletion {
    if (request !== this.activeRequest) return { type: "ignored" };
    const binding = this.bindAccount(request, actualAccountKey);
    if (binding.type === "superseded") {
      return binding;
    }

    let loaded: unknown;
    try {
      loaded = JSON.parse(serializedData);
    } catch (error) {
      return this.fail(request, error);
    }
    if (typeof loaded !== "object" || loaded === null || Array.isArray(loaded)) {
      return this.fail(
        request,
        new Error("work item extra data must be a JSON object")
      );
    }

    const data = mergePendingExtraData(
      loaded as Record<string, unknown>,
      this.pendingEdits
    );
    this.activeRequest = null;
    this.accountKey = actualAccountKey;
    this.captureEdits = false;
    this.pendingEdits.clear();
    return { type: "loaded", data };
  }
}

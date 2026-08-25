import { describe, expect, it } from "vitest";
import { WorkItemExtraDataReload } from "./workItemExtraDataReload";

describe("WorkItemExtraDataReload", () => {
  it("retains same-account edits after a rejected load and through retry", () => {
    const reload = new WorkItemExtraDataReload();
    const first = reload.start("github.com\u0000first");
    reload.recordEdit("edited", { note: "pending" });

    expect(reload.fail(first.request, new Error("IPC rejected")).type).toBe(
      "failed"
    );
    const retry = reload.start("github.com\u0000first");
    expect(
      reload.finish(
        retry.request,
        "github.com\u0000first",
        JSON.stringify({ edited: { note: "stale" } })
      )
    ).toEqual({
      type: "loaded",
      data: { edited: { note: "pending" } },
    });
  });

  it("retains edits when an initially unscoped load is bound before failing", () => {
    const reload = new WorkItemExtraDataReload();
    const first = reload.start();
    reload.recordEdit("edited", { note: "pending" });
    reload.bindAccount(first.request, "github.com\u0000first");
    reload.fail(first.request, "IPC rejected");

    const retry = reload.start("github.com\u0000first");
    expect(
      reload.finish(retry.request, "github.com\u0000first", "{}")
    ).toEqual({
      type: "loaded",
      data: { edited: { note: "pending" } },
    });
  });

  it("retains same-account edits after malformed data and through retry", () => {
    const reload = new WorkItemExtraDataReload();
    const first = reload.start("github.com\u0000first");
    reload.recordEdit("edited", { note: "pending" });

    expect(
      reload.finish(first.request, "github.com\u0000first", "{not-json").type
    ).toBe("failed");
    const retry = reload.start("github.com\u0000first");
    expect(
      reload.finish(retry.request, "github.com\u0000first", "{}")
    ).toEqual({
      type: "loaded",
      data: { edited: { note: "pending" } },
    });
  });

  it("merges retained edits when a retry succeeds", () => {
    const reload = new WorkItemExtraDataReload();
    const first = reload.start("github.com\u0000first");
    reload.recordEdit("edited", { note: "pending" });
    reload.fail(first.request, "offline");
    reload.recordEdit("added", { note: "after failure" });

    const retry = reload.start("github.com\u0000first");
    expect(
      reload.finish(
        retry.request,
        "github.com\u0000first",
        JSON.stringify({ untouched: { note: "loaded" } })
      )
    ).toEqual({
      type: "loaded",
      data: {
        untouched: { note: "loaded" },
        edited: { note: "pending" },
        added: { note: "after failure" },
      },
    });
  });

  it("discards retained edits when another account supersedes the load", () => {
    const reload = new WorkItemExtraDataReload();
    const first = reload.start("github.com\u0000first");
    reload.recordEdit("first-only", { note: "must not leak" });
    reload.fail(first.request, "offline");

    const second = reload.start("github.com\u0000second");
    expect(second.superseded).toBe(true);
    expect(
      reload.finish(second.request, "github.com\u0000second", "{}")
    ).toEqual({ type: "loaded", data: {} });
  });
});

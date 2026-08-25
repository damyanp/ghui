import { describe, expect, it } from "vitest";
import { WorkItemExtraDataReload } from "./workItemExtraDataReload";

describe("WorkItemExtraDataReload", () => {
  it("retains same-account edits after a rejected load and through retry", () => {
    const reload = new WorkItemExtraDataReload();
    const first = reload.start("github.com\u0000first");
    reload.recordEdit(
      "edited",
      { note: "pending" },
      "github.com\u0000first"
    );

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
    reload.bindAccount(first.request, "github.com\u0000first");
    reload.recordEdit(
      "edited",
      { note: "pending" },
      "github.com\u0000first"
    );
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
    reload.recordEdit(
      "edited",
      { note: "pending" },
      "github.com\u0000first"
    );

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
    reload.recordEdit(
      "edited",
      { note: "pending" },
      "github.com\u0000first"
    );
    reload.fail(first.request, "offline");
    reload.recordEdit(
      "added",
      { note: "after failure" },
      "github.com\u0000first"
    );

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
    reload.recordEdit(
      "first-only",
      { note: "must not leak" },
      "github.com\u0000first"
    );
    reload.fail(first.request, "offline");

    const second = reload.start("github.com\u0000second");
    expect(second.superseded).toBe(true);
    expect(
      reload.finish(second.request, "github.com\u0000second", "{}")
    ).toEqual({ type: "loaded", data: {} });
  });

  it("does not treat the first known account as a supersession", () => {
    const reload = new WorkItemExtraDataReload();
    reload.start();

    const sameAccount = reload.start("github.com\u0000first");

    expect(sameAccount.superseded).toBe(false);
  });

  it("rejects an edit captured by an editor for a previous account", () => {
    const reload = new WorkItemExtraDataReload();
    reload.start("github.com\u0000first");
    reload.start("github.com\u0000second");

    expect(
      reload.recordEdit(
        "first-only",
        { note: "must not leak" },
        "github.com\u0000first"
      )
    ).toBe(false);
  });

  it("ignores pending completion and edits after disposal", () => {
    const reload = new WorkItemExtraDataReload();
    const active = reload.start("github.com\u0000first");
    reload.dispose();

    expect(
      reload.finish(active.request, "github.com\u0000first", "{}")
    ).toEqual({ type: "ignored" });
    expect(
      reload.recordEdit(
        "item",
        { note: "must not apply" },
        "github.com\u0000first"
      )
    ).toBe(false);
  });

  it("cancels an unscoped load when no account is selected", () => {
    const reload = new WorkItemExtraDataReload();
    const active = reload.start();
    reload.cancel(active.request);

    expect(reload.finish(active.request, "github.com\u0000first", "{}")).toEqual(
      { type: "ignored" }
    );
  });

  it("supports re-driving a post-bind account supersession", () => {
    const reload = new WorkItemExtraDataReload();
    const stale = reload.start("github.com\u0000first");

    expect(
      reload.finish(stale.request, "github.com\u0000second", "{}")
    ).toEqual({ type: "superseded" });

    const recovery = reload.start("github.com\u0000second");
    expect(
      reload.finish(recovery.request, "github.com\u0000second", "{}")
    ).toEqual({ type: "loaded", data: {} });
  });

  it("invalidates a stale generation only while its request is current", () => {
    const reload = new WorkItemExtraDataReload();
    const stale = reload.start("github.com\u0000first");

    expect(reload.invalidateForAccountChange(stale.request)).toBe(true);
    expect(reload.invalidateForAccountChange(stale.request)).toBe(false);
    expect(
      reload.finish(stale.request, "github.com\u0000first", "{}")
    ).toEqual({ type: "ignored" });
  });
});

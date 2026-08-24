import { describe, expect, it } from "vitest";
import { RequestScopedEdits } from "./requestScopedEdits";

describe("RequestScopedEdits", () => {
  it("returns edits for the active request", () => {
    const edits = new RequestScopedEdits<string, string>();
    edits.begin(1);
    edits.set(1, "item", "updated");

    expect([...edits.take(1)]).toEqual([["item", "updated"]]);
  });

  it("does not carry edits into a superseding request", () => {
    const edits = new RequestScopedEdits<string, string>();
    edits.begin(1);
    edits.set(1, "old-account-item", "updated");
    edits.begin(2);
    edits.set(2, "new-account-item", "updated");

    expect([...edits.take(1)]).toEqual([]);
    expect([...edits.take(2)]).toEqual([["new-account-item", "updated"]]);
  });
});

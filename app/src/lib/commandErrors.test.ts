import { describe, expect, it } from "vitest";
import { commandErrorLogEntry } from "./commandErrors";

describe("commandErrorLogEntry", () => {
  it("formats Error instances for the frontend log", () => {
    expect(
      commandErrorLogEntry(
        "Failed to save work item extra data",
        new Error("account changed"),
        "12:34:56.789"
      )
    ).toEqual({
      timestamp: "12:34:56.789",
      level: "error",
      message: "Failed to save work item extra data: account changed",
    });
  });

  it("formats non-Error rejection values", () => {
    expect(commandErrorLogEntry("Save failed", "disk full", "now").message).toBe(
      "Save failed: disk full"
    );
  });
});

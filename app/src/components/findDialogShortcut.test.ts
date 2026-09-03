import { describe, expect, it } from "vitest";
import { isFindShortcut } from "./findDialogShortcut";

function keyboardEvent(
  key: string,
  modifiers: Partial<
    Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey">
  > = {}
) {
  return {
    key,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...modifiers,
  };
}

describe("isFindShortcut", () => {
  it("matches Ctrl+F", () => {
    expect(isFindShortcut(keyboardEvent("f", { ctrlKey: true }))).toBe(true);
    expect(isFindShortcut(keyboardEvent("F", { ctrlKey: true }))).toBe(true);
  });

  it("does not match ordinary typing", () => {
    expect(isFindShortcut(keyboardEvent("a"))).toBe(false);
    expect(isFindShortcut(keyboardEvent("f"))).toBe(false);
  });

  it("does not match other Ctrl shortcuts", () => {
    expect(isFindShortcut(keyboardEvent("c", { ctrlKey: true }))).toBe(false);
  });

  it("does not match Ctrl+F with additional modifiers", () => {
    expect(
      isFindShortcut(keyboardEvent("f", { ctrlKey: true, altKey: true }))
    ).toBe(false);
    expect(
      isFindShortcut(keyboardEvent("f", { ctrlKey: true, shiftKey: true }))
    ).toBe(false);
    expect(
      isFindShortcut(keyboardEvent("f", { ctrlKey: true, metaKey: true }))
    ).toBe(false);
  });
});

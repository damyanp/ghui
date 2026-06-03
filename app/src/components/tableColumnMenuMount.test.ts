import { describe, expect, it } from "vitest";

// Guards against the skeleton-svelte v4 regression where the column header menu
// rendered its body eagerly. Popover.Content in v4 mounts (and keeps mounted)
// its children regardless of open state, so the filter menu used to:
//   * freeze an empty option list (it `untrack`s field.options at mount, before
//     the project fields have loaded), and
//   * never flush staged filter changes (the child flushes on unmount, which
//     never happened while the menu stayed mounted).
// TableColumnMenu must therefore only render the menu body while `open`, so the
// menu mounts fresh on open and unmounts (flushing) on close.

const sources = import.meta.glob("./TableColumnMenu.svelte", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("TableColumnMenu only mounts its menu body while open", () => {
  const source = Object.values(sources)[0];

  it("loads the component source", () => {
    expect(source).toBeTruthy();
  });

  it("gates renderMenuContent behind an {#if open} block", () => {
    const gateIndex = source.indexOf("{#if open}");
    const renderIndex = source.indexOf(
      "props.column.renderMenuContent(props.column)"
    );
    expect(gateIndex).toBeGreaterThanOrEqual(0);
    expect(renderIndex).toBeGreaterThanOrEqual(0);
    // The menu body (which includes the renderMenuContent call) must live
    // inside the `{#if open}` gate so it mounts/unmounts with the popover.
    expect(gateIndex).toBeLessThan(renderIndex);
  });

  it("does not render a Popover.Arrow (it rendered as a stray dark diamond)", () => {
    // The skeleton-svelte v4 migration placed Popover.Arrow/ArrowTip inside
    // Popover.Content, which rendered as a detached black diamond near the
    // column header. The arrow is purely decorative and was removed.
    expect(source).not.toMatch(/<Popover\.Arrow[\s>]/);
    expect(source).not.toMatch(/<Popover\.ArrowTip[\s/>]/);
  });
});

import { describe, expect, it } from "vitest";

// Guards against the skeleton-svelte v4 migration regression where some
// components were left using the removed monolithic `<Switch>...{label}...</Switch>`
// API. In v4 a bare `<Switch>` (the Root) renders only an empty `<label>` with no
// visible control, which is what broke the single-select column filter menus
// (Kind/Status/Workstream/etc.) — the toggles disappeared and the filter list
// looked empty. Every Skeleton `Switch` must use the compound anatomy
// (`Switch.Control` + `Switch.Thumb` + `Switch.HiddenInput`).

// Read every component's source as a raw string via Vite (no node:fs needed, so
// this stays within the repo's "plain TS, runs in Node" test convention).
const sources = import.meta.glob("../**/*.svelte", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

function stripComments(source: string): string {
  // Drop HTML/Svelte comments so commented-out markup can't create false
  // matches in either direction.
  // Apply repeatedly to avoid incomplete multi-character sanitization where
  // comment delimiters can reappear after a single replacement pass.
  let current = source;
  let previous: string;
  do {
    previous = current;
    current = current.replace(/<!--[\s\S]*?-->/g, "");
  } while (current !== previous);
  return current;
}

function countMatches(source: string, re: RegExp): number {
  return (source.match(re) ?? []).length;
}

const importsSkeletonSwitch = (source: string): boolean =>
  /import\s*\{[^}]*\bSwitch\b[^}]*\}\s*from\s*["']@skeletonlabs\/skeleton-svelte["']/.test(
    source
  );

// Root `<Switch` but not `<Switch.Foo` subcomponents.
const ROOT_SWITCH = /<Switch(?=[\s>])/g;
const CONTROL = /<Switch\.Control(?=[\s>])/g;
const THUMB = /<Switch\.Thumb(?=[\s/>])/g;
const HIDDEN_INPUT = /<Switch\.HiddenInput(?=[\s/>])/g;

describe("Skeleton Switch usages use the compound anatomy", () => {
  const files = Object.entries(sources).filter(([, source]) =>
    importsSkeletonSwitch(source)
  );

  it("finds the components that use Switch", () => {
    // Sanity check so this suite can never silently pass by scanning nothing.
    expect(files.length).toBeGreaterThan(0);
  });

  for (const [path, raw] of files) {
    it(`${path} renders a control/thumb/hidden-input for every root Switch`, () => {
      const source = stripComments(raw);
      const roots = countMatches(source, ROOT_SWITCH);
      expect(roots).toBeGreaterThan(0);
      expect(countMatches(source, CONTROL)).toBeGreaterThanOrEqual(roots);
      expect(countMatches(source, THUMB)).toBeGreaterThanOrEqual(roots);
      expect(countMatches(source, HIDDEN_INPUT)).toBeGreaterThanOrEqual(roots);
    });
  }
});

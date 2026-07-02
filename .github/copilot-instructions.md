# Copilot Instructions for ghui

## Project Overview

ghui is a **GitHub project management desktop app** built with **Tauri 2 (Rust backend) + SvelteKit (Svelte 5 frontend)**. It uses the GitHub GraphQL API to manage project work items (issues, PRs) with features like bulk editing, sanitization rules, undo/redo, and drag-and-drop hierarchy management.

## Architecture

```
ghui/
├── github-graphql/     # Pure Rust library: GraphQL client, data models, change tracking
├── ghui-app/           # Core app logic: AppState, Tauri command implementations
├── app/src-tauri/      # Tauri shell: IPC bridge, window management, gh auth status
├── app/src/            # Svelte 5 frontend: components, routes, styling
└── ghui-util/          # Standalone CLI tool for maintenance tasks
```

### Key Design Decisions

- **`Changes` is a plain data container** — serializable, exported to TypeScript, compared for equality. It must not contain workflow state.
- **`UndoHistory` is separate from `Changes`** — it lives in `AppState` and is an editing-workflow concern. Never modify `Changes` directly in app state; always go through `UndoHistory.track_*()` methods.
- **Data flows one way**: Frontend calls Tauri commands → Rust modifies state → watcher callback pushes `DataUpdate` back to frontend.
- **Caching**: Fields and WorkItems are cached to `~/{name}.ghui.json`. Try cache first; hit GitHub API only on `force_refresh=true`.

## Build, Test, and Lint Commands

### Rust

```bash
# Build (requires system deps on Linux: libdbus-1-dev, libwebkit2gtk-4.1-dev)
cargo build --verbose

# Run all tests
cargo test --verbose

# Run just the core library tests (fastest, no system deps needed)
cargo test -p github-graphql

# Format (must pass before merge)
cargo fmt --all -- --check

# Clippy (treat warnings as errors, must pass before merge)
cargo clippy --all -- -D warnings
```

### Frontend (Svelte/TypeScript)

```bash
cd app

# Install dependencies
npm ci

# Type checking
npm run check

# Run frontend unit tests (vitest)
npm test

# Production build — the ONLY check that compiles the Tailwind/Skeleton CSS
# bundle. Always run this after dependency bumps (see note below).
npm run build

# Dev server
npm run dev
```

**Verifying dependency updates (esp. major bumps).** `npm run check` and
`npm test` do not compile the production CSS bundle, so a broken Tailwind or
Skeleton `@import`/`@utility` will pass both yet fail `npm run build`. This is
exactly how a dependabot Skeleton 3→4 major bump shipped a broken installer
build. Whenever you touch `app/package.json` / `package-lock.json` — or review
a dependabot PR — run a clean `npm ci` (so you get the locked versions, not
stale `node_modules`) followed by `npm run build`, and check the dependency's
migration guide for major bumps. In Skeleton v4 the `./optional/*` export
(e.g. `@skeletonlabs/skeleton/optional/presets`) was removed; presets are now
bundled into the main `@import '@skeletonlabs/skeleton'`.

Frontend tests live next to the code they cover as `*.test.ts` files (e.g.
`app/src/lib/filterableFields.test.ts`). Tests use **vitest** and should be
plain TypeScript — keep them dependency-free of Svelte/Tauri runtime so they
run fast in Node. When you add or refactor logic in `app/src/lib/**`,
**always add or update a `*.test.ts` next to it**. Pure helper modules (no
Svelte/Tauri imports) are the easiest to test; if you find yourself wanting to
test logic that lives inside a `.svelte.ts` class, extract it into a pure
helper module and have the class delegate to it (this is how
`WorkItemContext` consumes `filterableFields.ts`).

### Full Tauri app

```bash
cd app
npm ci
npx tauri dev     # Development
npx tauri build   # Release build (produces MSI + NSIS installers)
```

### CI

- **rust.yml**: Runs `cargo fmt --check`, `cargo clippy`, `cargo test`, `npm run check`, `npm test` (vitest), and `npm run build` (production frontend build) on `windows-latest` for push/PR to main.
- **build-installer.yml**: Builds Windows installer on push to main.

The CI runs on Windows. The Copilot cloud agent environment is Linux, and `.github/workflows/copilot-setup-steps.yml` preinstalls the GTK/WebKit/glib `-dev` packages that `app/src-tauri` links against, so `cargo clippy --all -- -D warnings` should work end-to-end. If you ever find those packages missing (for example, in a fresh local Linux environment), install them with:

```bash
sudo apt-get update && sudo apt-get install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev libdbus-1-dev libglib2.0-dev libgtk-3-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev build-essential pkg-config
```

Do **not** fall back to `cargo clippy -p github-graphql` as a substitute for `cargo clippy --all` — install the deps and run the full workspace check.

## Rust Conventions

### Types and Derivations

```rust
#[derive(Default, Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MyType { ... }
```

- All types exposed to the frontend must derive `TS` and have `#[ts(export)]`.
- Use `#[serde(rename_all = "camelCase")]` so Rust `snake_case` fields become TypeScript `camelCase`.
- Common derive set: `Default`, `Serialize`, `Deserialize`, `Clone`, `Debug`, `TS`.

### Error Handling

- Use `anyhow::Result<T>` for internal operations.
- Tauri commands return `TauriCommandResult<T>` (wraps `anyhow::Error` for serialization).
- Use the `?` operator for propagation.
- **Avoid `.expect()` and `.unwrap()` in non-test code.** Use graceful error handling (`Result`, fallback behavior, or log + continue). Panicking should only happen for truly unrecoverable states, never for file I/O, environment lookups, or optional features like logging.

### Idiomatic Patterns

- **Use let chains** for multiple `if let` conditions — prefer `if let ... && let ...` over tuple destructuring:
  ```rust
  // Good: let chains
  if let Some(a) = x.as_ref()
      && let Some(b) = y.as_ref()
  {
      // ...
  }

  // Bad: tuple pattern match
  if let (Some(a), Some(b)) = (x.as_ref(), y.as_ref()) {
      // ...
  }
  ```
- **Prefer existing crates over hand-rolling utilities.** Before implementing utility functionality (date formatting, path manipulation, string processing, etc.), check if an existing dependency or a transitive dependency already provides it.

### Tauri Command Pattern

```rust
#[tauri::command]
pub async fn my_command(
    data_state: State<'_, DataState>,
    arg: MyArg,
) -> TauriCommandResult<()> {
    data_state.lock().await.my_method(arg).await?;
    Ok(())
}
```

- Always inject `State<'_, DataState>` and `.lock().await` for async mutex access.
- Register in `tauri::generate_handler![...]` in `app/src-tauri/src/lib.rs`.

### Test Conventions

- **Always write tests for new functionality.** If you add a new function, method, or behavior path, write tests covering the happy path and key edge cases before submitting.
- **Name format**: `test_<action>_<scenario>` (e.g., `test_undo_add_change`).
- **Builder pattern** for test data:
  ```rust
  let mut data = TestData::new();
  let id = data.build().status("Active").assignees(vec!["user"]).add();
  ```
- Use `assert_eq!` and `assert!` for assertions.
- Snapshot testing with `insta` is available in `github-graphql`.
- Tests live in `github-graphql/src/data/tests.rs` with helpers in `test_helpers.rs`.
- NodeBuilder tests live in `ghui-app/src/nodes.rs` (run with `cargo test -p ghui-app`; requires the Tauri system deps on Linux — see the apt-get command in the Build/Test/Lint section).

#### Frontend tests (vitest)

- Frontend tests live next to the code as `*.test.ts` files (e.g.
  `app/src/lib/filterableFields.test.ts`) and run with `cd app && npm test`.
- They run in plain Node — keep them free of Svelte/Tauri runtime imports.
  `invoke` from `@tauri-apps/api/core` is not available in tests.
- To make `.svelte.ts` class logic testable, extract the pure logic into a
  sibling module (no runes, no `invoke`) and have the class delegate to it.
  This is how `WorkItemContext` consumes `filterableFields.ts`, and is the
  pattern to follow for any new testable logic.
- Pinning tests for refactors: when refactoring shared logic, add a
  `*.test.ts` that locks in the pre-refactor behavior across all relevant
  cases (e.g., `filterableFields.test.ts` covers all 8 filterable fields and
  both raw / `DelayLoad`-wrapped value shapes) before changing the
  implementation.

### WorkItems::update() and UpdateType

The `WorkItems::update()` method handles incremental updates when items change on GitHub. It returns an `UpdateType` that drives the refresh behavior:

- `NoUpdate` — nothing changed, no UI refresh needed.
- `SimpleChange` — only non-hierarchy fields changed (e.g., title); the item is pushed directly to the frontend via `DataUpdate::WorkItem`.
- `ChangesHierarchy` — structural changes occurred (e.g., assignees, status, sub-issues, parent); triggers a full `refresh(false)` to rebuild the node tree.

When modifying `update()` or `get_work_item_update_type()`:
1. **New items must be added to `ordered_items`** — the HashMap alone isn't enough; `get_roots()` and `iter()` depend on `ordered_items`.
2. **Add tests** for any new `UpdateType` classification logic in `github-graphql/src/data/tests.rs`.
3. **Add a NodeBuilder test** in `ghui-app/src/nodes.rs` to verify that items appear correctly in the node tree after updates.

### Sanitize Rules

The `sanitize()` method in `work_items.rs` returns a `Changes` struct (rules are not applied directly). When adding new sanitize rules:
1. Add the rule logic in `sanitize()` or `sanitize_issue_hierarchy()`.
2. Add tests covering the new rule and edge cases (closed items, existing status, etc.).
3. Follow the existing pattern: check conditions → emit `Change` with appropriate `ChangeData`.

### Logging

- Use the `log` crate macros (`info!`, `warn!`, `error!`, `debug!`) in `github-graphql` and `ghui-app`. Keep `println!` for user-facing CLI output in `ghui-util`.
- **Never log full API response bodies at `error!` or `warn!` level** — they can be large and may contain sensitive project/issue data. Log only error metadata (status code, byte count, error message) at higher levels. Raw response content should only appear at `debug!` level, truncated to 1024 chars total.
- When writing to log files or any file where immediate persistence matters, use direct `File` writes (not `BufWriter`) or ensure explicit flushing. `BufWriter` without `flush()` will buffer indefinitely.

### Workspace version bumps and `Cargo.lock`

When any workspace crate's `Cargo.toml` `[package]` version changes, `Cargo.lock` **must** be regenerated and committed in the same change. The lockfile records the resolved versions of every workspace crate, and a manifest bump without a lockfile bump leaves the repo in an inconsistent state — CI builds that touch those crates will then produce a "spurious" `Cargo.lock` diff on every subsequent PR until someone fixes it (this is what caused PRs #42 and #44).

Rules:

- **After bumping any workspace crate version**, run `cargo update --workspace` (or simply `cargo build` / `cargo check`) and commit the resulting `Cargo.lock` change in the same commit/PR as the version bump.
- **Never blindly revert an "unrelated" `Cargo.lock` diff** that appears after running cargo commands. First investigate whether the workspace `Cargo.toml` package versions match the lockfile's `[[package]]` entries for those crates. If they don't, the lockfile diff is real and required — keep it.
- Before reverting any `Cargo.lock` diff, run `git diff Cargo.toml */Cargo.toml` and inspect the lockfile entries for the workspace crates (`ghui`, `ghui-app`, `ghui-util`, `github-graphql`) to confirm versions actually match. Only revert if they already match in the lockfile.

### TypeScript Bindings (ts-rs)

- Bindings are regenerated by running `cargo test`. The `TS_RS_EXPORT_DIR` is set to `app/src/lib/bindings` in `.cargo/config.toml`.
- When regenerating bindings, run `cargo test --all` (not just a single package) to ensure all ts-rs exports across the workspace are regenerated. Verify `git diff` shows actual changes before creating a PR. If no files changed, the bindings were already up to date — don't create a no-op PR.

## Svelte / Frontend Conventions

### Svelte 5 Runes

This project uses **Svelte 5** with runes — not Svelte 4 stores:

```svelte
let mode = $state<Mode>("items");           // Mutable state
const numChanges = $derived(Object.keys(context.data.changes.data).length);  // Computed
let { columns = $bindable() } = $props();   // Two-way bindable props
```

### Component Patterns

- **Context**: Created in root `+page.svelte` with `setWorkItemContext()`, consumed in children.
- **Snippets** for template reuse: `{#snippet lead()} ... {/snippet}`.
- **Generics**: Components can be generic: `<script lang="ts" generics="T, GROUP, ITEM">`.
- **Attachments**: Use `{@attach handler}` for element lifecycle (e.g., IntersectionObserver).

### Styling

- **Tailwind CSS 4** with **Skeleton UI** (Cerberus theme).
- **Always reuse existing theme colors** — use tokens like `bg-primary-100-900`, `bg-surface-300-700` rather than custom color values.
- Don't add CSS comments with specific color values; they go stale.
- Dark mode support: use Skeleton's `X-Y` pattern (e.g., `bg-primary-100-900` picks light/dark automatically).
- Icons: `@lucide/svelte` for UI icons, `@primer/octicons` for GitHub-specific icons.

### Drag / Resize Interactions

- Use **Pointer Events** with `setPointerCapture`/`releasePointerCapture` and element-level `onpointermove`/`onpointerup` handlers.
- Do **not** attach `pointermove`/`pointerup` listeners on `document` — this can leak listeners if the component unmounts mid-drag.

### Toolbar / AppBar

- All toolbar buttons use `AppBarButton` component.
- Buttons are always visible and disabled when they can't be used (don't conditionally show/hide buttons).
- Group related buttons with gaps between groups.

## General Guidance for Agents

### Do

- **Reuse existing patterns** — look at how similar things are done elsewhere in the codebase before introducing new approaches.
- **Keep `Changes` and `UndoHistory` separate** — this was a deliberate architectural decision.
- **Add comments for non-obvious behavior** (e.g., why the window starts invisible).
- **Run the full validation suite before every commit that changes Rust or frontend code.** All six checks must be run together as a set — never pick and choose. Running clippy without fmt (or vice versa) is the most common cause of CI failures on this repo (see PR #44):
  1. `cargo fmt --all -- --check`
  2. `cargo clippy --all -- -D warnings`
  3. `cargo test -p github-graphql` (or `cargo test --all` if ts-rs bindings may need regeneration)
  4. `cd app && npm run check`
  5. `cd app && npm test`
  6. `cd app && npm run build`

  After making code changes, run all six. If any fail, fix and re-run the full set — don't assume the others still pass. Note that `npm run check` (svelte-check) and `npm test` (vitest) do **not** compile the production CSS bundle, so only `npm run build` catches Tailwind/Skeleton `@import`/`@utility` errors (this is what let the Skeleton 3→4 bump ship a broken installer build — see below).
- **Explicitly report validation results in PR comments.** Whenever you reply to the user or call `report_progress` after code changes, list each of the five commands above with a pass/fail indicator (e.g. ✅/❌). The reviewer should never have to ask "is this clippy clean?" or "will this pass CI?" — that information should already be in your last comment.
- **Think independently** about suggestions — evaluate whether a proposed change actually makes sense before implementing it.
- **Remove unnecessary code/config** — don't add things "just in case" (e.g., don't add `center: true` if the window starts hidden).
- **Apply cross-cutting concerns consistently** — when adding a pattern like a busy guard, loading state, or error-handling wrapper, apply it to ALL relevant handlers/callsites, not just one. Audit the full set of similar code paths.
- **Update the PR title and description after review-driven changes** — if review feedback significantly changes the approach or scope, update the PR metadata to reflect the current state.
- **When responding in a PR comment after making changes, list the validation commands you ran and their pass/fail results**, so the reviewer doesn't have to ask. See the validation block above for the required set.

### Posting screenshots in PR comments

Screenshots posted to PRs must actually render for the reviewer. Past PRs (e.g. #41) have repeatedly posted broken images that the user could not see. Follow these rules:

- **Never post local sandbox file paths** (e.g. `/tmp/foo.png`, or any path on the agent VM) as screenshots — those paths are not accessible to anyone outside the sandbox and will appear as broken links or plain text.
- **Prefer `https://github.com/user-attachments/assets/...` URLs** produced by uploading the image to a GitHub comment. These are the most reliable way to embed screenshots in PR comments.
- **Avoid `raw.githubusercontent.com/<owner>/<repo>/<sha>/...` links** to files committed in the repo — these have rendered as blank images in practice on this repo.
- If you must reference an image stored in the repo, link to its **GitHub blob URL** (`https://github.com/<owner>/<repo>/blob/<branch-or-sha>/path/to/image.png`) rather than a `raw.githubusercontent.com` URL pinned to a commit.

### Don't

- Don't add comments with specific values that will go stale (e.g., "sets color to #1a2b3c").
- Don't introduce custom CSS color definitions when theme tokens exist.
- Don't put undo/redo state inside the `Changes` struct.
- Don't conditionally show/hide toolbar buttons — keep them visible and disable them instead.
- Don't skip clippy, fmt, or test validation (Rust **or** frontend `npm test`).
- Don't add UI elements (buttons, menu items, panels) until they have working functionality. Placeholder/stub UI creates a confusing user experience.
- Don't use `.expect()` or `.unwrap()` in non-test code — see [Error Handling](#error-handling).

### Environment Notes

- The `gh` CLI must be installed, on `PATH`, and authenticated (`gh auth login`) for runtime. All GitHub API calls go through `gh api graphql` via `GhCliClient`.
- The `graphql.config.yml` points to the GraphQL schema and query documents for IDE support.
- VS Code extensions: `svelte.svelte-vscode`, `tauri-apps.tauri-vscode`, `rust-lang.rust-analyzer`.
- Rust Analyzer uses a separate target dir (`target/analyzer`) to avoid conflicts with cargo builds.

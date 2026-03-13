# Copilot Instructions for ghui

## Project Overview

ghui is a **GitHub project management desktop app** built with **Tauri 2 (Rust backend) + SvelteKit (Svelte 5 frontend)**. It uses the GitHub GraphQL API to manage project work items (issues, PRs) with features like bulk editing, sanitization rules, undo/redo, and drag-and-drop hierarchy management.

## Architecture

```
ghui/
├── github-graphql/     # Pure Rust library: GraphQL client, data models, change tracking
├── ghui-app/           # Core app logic: AppState, Tauri command implementations
├── app/src-tauri/      # Tauri shell: IPC bridge, window management, PAT storage
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

# Clippy (treat warnings as errors)
cargo clippy -- -D warnings
```

### Frontend (Svelte/TypeScript)

```bash
cd app

# Install dependencies
npm ci

# Type checking
npm run check

# Dev server
npm run dev
```

### Full Tauri app

```bash
cd app
npm ci
npx tauri dev     # Development
npx tauri build   # Release build (produces MSI + NSIS installers)
```

### CI

- **rust.yml**: Runs `cargo build` and `cargo test` on `windows-latest` for push/PR to main.
- **build-installer.yml**: Builds Windows installer on push to main.

The CI runs on Windows. If you can't run a full `cargo build` locally (missing system deps on Linux), validate with `cargo test -p github-graphql` and `cd app && npm run check`.

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

- **Name format**: `test_<action>_<scenario>` (e.g., `test_undo_add_change`).
- **Builder pattern** for test data:
  ```rust
  let mut data = TestData::new();
  let id = data.build().status("Active").assignees(vec!["user"]).add();
  ```
- Use `assert_eq!` and `assert!` for assertions.
- Snapshot testing with `insta` is available in `github-graphql`.
- Tests live in `github-graphql/src/data/tests.rs` with helpers in `test_helpers.rs`.
- NodeBuilder tests live in `ghui-app/src/nodes.rs` (run with `cargo test -p ghui-app`, requires `libdbus-1-dev` on Linux).

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

### Toolbar / AppBar

- All toolbar buttons use `AppBarButton` component.
- Buttons are always visible and disabled when they can't be used (don't conditionally show/hide buttons).
- Group related buttons with gaps between groups.

## General Guidance for Agents

### Do

- **Reuse existing patterns** — look at how similar things are done elsewhere in the codebase before introducing new approaches.
- **Keep `Changes` and `UndoHistory` separate** — this was a deliberate architectural decision.
- **Add comments for non-obvious behavior** (e.g., why the window starts invisible).
- **Run validation before submitting**: `cargo test -p github-graphql`, `cargo clippy -- -D warnings`, `cd app && npm run check`.
- **Think independently** about suggestions — evaluate whether a proposed change actually makes sense before implementing it.
- **Remove unnecessary code/config** — don't add things "just in case" (e.g., don't add `center: true` if the window starts hidden).

### Don't

- Don't add comments with specific values that will go stale (e.g., "sets color to #1a2b3c").
- Don't introduce custom CSS color definitions when theme tokens exist.
- Don't put undo/redo state inside the `Changes` struct.
- Don't conditionally show/hide toolbar buttons — keep them visible and disable them instead.
- Don't skip clippy or test validation.

### Environment Notes

- A GitHub PAT with "project" scope is required for runtime. See `.env-example`.
- The `graphql.config.yml` points to the GraphQL schema and query documents for IDE support.
- VS Code extensions: `svelte.svelte-vscode`, `tauri-apps.tauri-vscode`, `rust-lang.rust-analyzer`.
- Rust Analyzer uses a separate target dir (`target/analyzer`) to avoid conflicts with cargo builds.

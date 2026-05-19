# Project Context

- **Owner:** Damyan Pepper
- **Project:** ghui — Tauri 2 + SvelteKit GitHub project management desktop app
- **Stack:** Rust (Tauri 2, anyhow, ts-rs, insta), TypeScript (Svelte 5 runes, Tailwind 4 / Skeleton, vitest), GitHub GraphQL
- **Current focus:** Pivoting plan Phase 2 — Tasks 2 (`RecipeNodeBuilder`, PR #70 draft), 4 (`PivotConfig` in AppState + Tauri commands, PR #72 ready). Possibly Task 6 wire-up after Phase 2 lands.
- **Created:** 2026-05-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->

- 📌 Repo layout: `github-graphql/` (pure Rust lib, data models, parser), `ghui-app/` (AppState + Tauri command impls), `app/src-tauri/` (Tauri shell + IPC registration), `app/src/` (Svelte frontend), `ghui-util/` (CLI).
- 📌 Architectural rule: `Changes` is a plain data container (serializable, TS-exported, equality-compared). `UndoHistory` is a separate concern living in `AppState`. Never modify `Changes` directly — go through `UndoHistory.track_*()`.
- 📌 ts-rs export dir is `app/src/lib/bindings` (set via `TS_RS_EXPORT_DIR` in `.cargo/config.toml`). `cargo test --all` regenerates all bindings; `cargo test -p github-graphql` only does that crate.
- 📌 Frontend type derives: `#[derive(Default, Serialize, Deserialize, TS, Debug, Clone)]` + `#[serde(rename_all = "camelCase")]` + `#[ts(export)]`.
- 📌 Tauri command pattern: `State<'_, DataState>`, `data_state.lock().await`, return `TauriCommandResult<T>`. Register in `tauri::generate_handler![...]` in `app/src-tauri/src/lib.rs`.
- 📌 Caching: Fields and WorkItems cache to `~/{name}.ghui.json`. Try cache first; hit GitHub API only on `force_refresh=true`. `ViewConfigCache` (filters + pivot_config) persists to `~/view_config.ghui.json`.
- 📌 `WorkItems::update()` returns `UpdateType`: `NoUpdate`, `SimpleChange` (push direct via `DataUpdate::WorkItem`), `ChangesHierarchy` (full `refresh(false)`). New items MUST be added to `ordered_items` not just the HashMap.
- 📌 Sanitize rules: `sanitize()` returns a `Changes` struct (not applied directly). Add tests for new rules.
- 📌 Logging: use `log` crate macros. NEVER log full API response bodies at `error!`/`warn!` — debug-level only, truncated to 1024 chars. Use direct `File` writes (not `BufWriter`) where immediate persistence matters.
- 📌 Pivoting Task 1 (foundation) added `github-graphql/src/pivot.rs` with `PivotField`, `Axis`, `MultiValueStrategy`, `PivotConfig`, plus `parse_recipe()` / `recipe_to_string()`. Fixture: `github-graphql/tests/fixtures/recipes.json` — this is the contract between Tasks 2 and 5.
- 📌 Phase 2 additive rule: NEVER modify `NodeBuilder::add_nodes()` or `WorkItemTree.svelte` rendering logic — reserved for Task 6.
- 📌 Team update (2026-05-19): PR #72 (Task 4) review revealed plan spec premise was incomplete. Plan said "persist to the same per-project cache file that Filters already writes to" — but Filters had NO persistence path before Task 4. Author correctly invented `ViewConfigCache` + `~/view_config.ghui.json` (flat-file convention matching `fields.ghui.json`, `work_items.ghui.json`). Basher: amend `docs/pivoting-implementation-plan.md` Task 4 section to note: (1) Filters now persist via `ViewConfigCache` as side effect, (2) cache file is `~/view_config.ghui.json` (not per-project named).

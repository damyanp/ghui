# Squad Team

> ghui — Tauri 2 + SvelteKit GitHub project management desktop app.

## Coordinator

| Name | Role | Notes |
|------|------|-------|
| Squad | Coordinator | Routes work, enforces handoffs and reviewer gates. Does not generate domain artifacts. |

## Members

| Name | Role | Charter | Status |
|------|------|---------|--------|
| Rusty | Lead | `.squad/agents/rusty/charter.md` | ✅ Active |
| Basher | Rust / Backend | `.squad/agents/basher/charter.md` | ✅ Active |
| Linus | Frontend | `.squad/agents/linus/charter.md` | ✅ Active |
| Livingston | Tester | `.squad/agents/livingston/charter.md` | ✅ Active |
| Scribe | Session Logger | `.squad/agents/scribe/charter.md` | 📋 Silent |
| Ralph | Work Monitor | `.squad/agents/ralph/charter.md` | 🔄 Monitor |

## Coding Agent

<!-- copilot-auto-assign: false -->

| Name | Role | Charter | Status |
|------|------|---------|--------|
| @copilot | Coding Agent | `.copilot/copilot-instructions.md` + `.github/copilot-instructions.md` | 🤖 Active (already opening PRs) |

### Capabilities

**🟢 Good fit — auto-route when enabled:**
- Single-task implementation against a clearly-scoped issue with acceptance criteria (e.g., the pivoting Task 2–5 pattern)
- Test additions next to existing patterns (`TestData` builder, vitest pure helpers)
- Lint / format fixes, dependency bumps, ts-rs binding regeneration
- Documentation fixes and README updates

**🟡 Needs review — route to @copilot but flag for squad member PR review:**
- Phase 2 / Phase 4 task PRs — always reviewed by Rusty + Livingston before merge
- Refactors with existing test coverage where the contract is well-defined
- Sanitize-rule additions following the existing pattern

**🔴 Not suitable — route to squad member instead:**
- Architectural calls (e.g., "switch Task 5 from a TS parser to Tauri delegation") — Rusty owns these
- Multi-file changes that span the Phase 2 "additive only" rule into Task 6 territory
- `UndoHistory` / `Changes` boundary changes
- Watcher callback / `DataUpdate` flow changes
- Anything requiring a `Cargo.lock` desync investigation

## Project Context

- **Owner:** Damyan Pepper
- **Project:** ghui
- **Stack:** Rust (Tauri 2, anyhow, ts-rs, insta), TypeScript (Svelte 5 runes, Tailwind 4 / Skeleton, vitest), GitHub GraphQL
- **Description:** Desktop app for managing GitHub project work items (issues, PRs) with bulk editing, sanitization rules, undo/redo, drag-and-drop hierarchy management, and configurable pivot/grouping recipes.
- **Active plan:** `docs/pivoting-implementation-plan.md` — Phase 1 landed, Phase 2 (4 open PRs) in review, Phase 3 (wire-up) and Phase 4 (polish) issues open.
- **Created:** 2026-05-19

# Work Routing

How to decide who handles what.

## Routing Table

| Work Type | Route To | Examples |
|-----------|----------|----------|
| Rust / Tauri / backend | Basher | NodeBuilder, RecipeNodeBuilder, AppState, Tauri commands, GraphQL client, ts-rs bindings, fixtures |
| Frontend / Svelte / UI | Linus | RecipeBar, WorkItemTree, AppBarButton, toolbar, Tailwind/Skeleton tokens, vitest helpers |
| Testing / fixture parity | Livingston | Rust unit + insta snapshots, vitest, cross-language parity via `recipes.json`, edge cases |
| Code review / PR gates | Rusty | Review PRs, run the five-command validation suite, approve/reject, enforce additive-only rule |
| Scope / priorities / plan amendments | Rusty (escalate to Damyan) | Phase ordering, when to amend `pivoting-implementation-plan.md`, merge order conflicts |
| Architectural decisions | Rusty | `PivotConfig` shape, parser contract, `UndoHistory`/`Changes` boundary, watcher flow |
| Session logging | Scribe | Automatic — never needs routing |
| Backlog / PR queue monitoring | Ralph | "Ralph, go" / "Ralph, status" — scan open PRs and `squad:*` issues, drive next action |
| Autonomous issue pickup | @copilot | `squad:copilot` labeled issues fitting the 🟢 capability profile |

## Issue Routing

| Label | Action | Who |
|-------|--------|-----|
| `squad` | Triage: analyze issue, assign `squad:{member}` label | Rusty (Lead) |
| `squad:rusty` | Architectural call, plan amendment, conflict triage | Rusty |
| `squad:basher` | Rust / Tauri / backend implementation | Basher |
| `squad:linus` | Svelte / frontend implementation | Linus |
| `squad:livingston` | Tests, fixture parity, CI gate | Livingston |
| `squad:copilot` | Autonomous PR by @copilot — needs squad review before merge | @copilot, reviewed by Rusty + Livingston |
| `pivoting` | Pivoting plan work — see `docs/pivoting-implementation-plan.md` | Routed by phase / task |
| `phase-2` | Parallel implementation (additive only) | Basher / Linus, reviewed by Rusty + Livingston |
| `phase-3` | Wire-up (Task 6) — depends on Phase 2 landing | Basher + Linus jointly, gated by Rusty |
| `phase-4` | Polish (Tasks 7–9) — depends on Task 6 | Mostly Linus, with Basher for `Node` struct changes |

### How Issue Assignment Works

1. When a GitHub issue gets the `squad` label, **Rusty** triages it — analyzing content, assigning the right `squad:{member}` label, and commenting with triage notes.
2. When a `squad:{member}` label is applied, that member picks up the issue in their next session.
3. Members can reassign by removing their label and adding another member's label.
4. The `squad` label is the "inbox" — untriaged issues waiting for Rusty's review.

## Rules

1. **Eager by default** — spawn all agents who could usefully start work, including anticipatory downstream work.
2. **Scribe always runs** after substantial work, always as `mode: "background"`. Never blocks.
3. **Quick facts → coordinator answers directly.** Don't spawn an agent for "what port does the server run on?"
4. **When two agents could handle it**, pick the one whose domain is the primary concern.
5. **"Team, ..." → fan-out.** Spawn all relevant agents in parallel as `mode: "background"`.
6. **Anticipate downstream work.** If a feature is being built, spawn the tester to write test cases from requirements simultaneously.
7. **Issue-labeled work** — when a `squad:{member}` label is applied to an issue, route to that member. The Lead handles all `squad` (base label) triage.

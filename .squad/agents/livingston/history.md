# Project Context

- **Owner:** Damyan Pepper
- **Project:** ghui — Tauri 2 + SvelteKit GitHub project management desktop app
- **Stack:** Rust (Tauri 2, anyhow, ts-rs, insta), TypeScript (Svelte 5 runes, Tailwind 4 / Skeleton, vitest), GitHub GraphQL
- **Current focus:** Pivoting plan PR review for tests. All four Phase 2 PRs (#70, #71, #72, #73) need test review — coverage, fixture parity, validation-suite pass. Cross-language parity via `github-graphql/tests/fixtures/recipes.json` is the critical contract.
- **Created:** 2026-05-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->

- 📌 Test naming: `test_<action>_<scenario>`.
- 📌 `TestData` builder: `data.build().status("Active").assignees(vec!["user"]).add()`. `WorkItemId` is incrementing numeric string starting "1" per TestData instance (`next_id` resets on `Default`).
- 📌 Snapshot testing with `insta` is available in `github-graphql`. NodeBuilder tests live in `ghui-app/src/nodes.rs` (run with `cargo test -p ghui-app`; requires `libdbus-1-dev` on Linux).
- 📌 Vitest tests: `app/src/lib/*.test.ts`. Plain TS, no Svelte/Tauri imports. `invoke` is not available — extract pure helpers from `.svelte.ts` classes and test those.
- 📌 Validation suite (all five together, every PR): `cargo fmt --all -- --check` → `cargo clippy --all -- -D warnings` → `cargo test --all` (or `-p github-graphql`) → `cd app && npm run check` → `cd app && npm test`.
- 📌 PRs #42 and #44 caused `Cargo.lock` desync. Investigate, never blindly revert. Confirm workspace `Cargo.toml` package versions match lockfile `[[package]]` entries before deciding the diff is "unrelated".
- 📌 Pivoting Task 1 (foundation, landed): parser in `github-graphql/src/pivot.rs`. Round-trip every preset from the prototype. Error cases: unknown field, unknown axis, missing parens. Snapshot the fixture.
- 📌 Task 5 deviated from a hand-rolled TS parser to delegating through Tauri (per PR #73 title) — verify cross-language parity is still tested somehow.
- 📌 Team update (2026-05-19): Phase 2 review batch complete. Parity-contract lesson: when parallel Task N and Task N+k deviations appear (async parser in #73 vs sync parser in #71), the fixture contract breaks if both land. Tests must explicitly validate the pair or refuse to merge one. Cross-parser parity is not implicit. On fixture-driven specs, add a "cross-impl agreement" test requirement. Rejection verdicts for #73: lost fixture coverage, hollow mocked tests, cross-PR collision with #71.
- 📌 Fixture-driven shim test pattern (PR #73, 2026-05-18): When a TS module delegates to a Tauri command, verify the shim by loading `github-graphql/tests/fixtures/*.json` with `fs.readFileSync` (plain Node, no build step), iterating over every entry, mocking `invoke` with `mockResolvedValueOnce`, calling the shim function, and asserting both the `invoke` call args and the returned value. Fixture path from `app/src/lib/` is `../../../github-graphql/tests/fixtures/` (3 levels up reaches the repo root). Use `import.meta.dirname` for reliable resolution in vitest. This replaces duplicating parse logic in TS tests — the Rust parser's own fixture tests handle parse correctness; the TS shim test only checks IPC wiring.
- 📌 vitest passes ≠ svelte-check passes; always run BOTH `npm test` AND `npm run check`. Node API imports break svelte-check unless `types: ['node']` is in tsconfig — prefer Vite JSON imports for fixtures.

# Wave A — Phase 2 PR fixes

**Date:** 2026-05-19
**Coordinator:** Squad
**Agents:** Basher (PR #72), Livingston (PR #73), Coordinator (PR #70)

## Outcomes
- PR #70: title fixed to convention, un-drafted. CI gated on Damyan's first-contributor approval.
- PR #72: test added (test_set_filters_preserves_pivot_config_in_cache), pushed 72bdda5. CI running.
- PR #73: fixture + error tests added; frontend type-check failed (Node imports vs. svelte-check). Follow-up fix in flight.

## Notes
- Local clippy on main fails with uninlined_format_args in telemetry.rs/updater.rs (toolchain drift; CI uses older Rust, passes there). Backlog item for Rusty.
- gh pr edit can fail silently with HTTP 1 due to projects-classic deprecation warning. Workaround: gh api -X PATCH repos/{owner}/{repo}/pulls/{n} --input -.
- First-time-contributor CI approval is web-UI only for same-repo PRs; /approve REST endpoint is fork-only.

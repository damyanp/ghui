# ghui UI declutter brief (for mockup review)

## Scope

Create review-first mockups for a less cluttered toolbar and clearer edit/save flow, based on prior log + telemetry analysis.

## Evidence highlights

- Most sessions are read-only and stay in **Items** mode.
- **Sanitize** is often clicked speculatively and frequently produces 0 changes.
- Save can feel long; progress feedback is present but easy to miss.
- **Add** and **Discard** are rarely used from the top bar.
- **Find** has usage but low discoverability.

## Mockup goals

1. Reduce top-bar density while keeping power-user actions available.
2. Group actions by task flow: **Load → Edit → Review → Save**.
3. Make "changes pending" and "save in progress" more visible.
4. Keep telemetry-visible behavior intact (mode switching, sanitize, save).

## Review package

- Interactive mockup route: `/test_ui_declutter`
- Three scenarios:
  - Read-only browsing
  - Active editing with pending changes
  - Conflict-heavy cleanup

## Non-goals

- No backend/command changes.
- No functional behavior change in main app route.

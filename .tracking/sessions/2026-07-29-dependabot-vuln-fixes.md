# Fixing reported Dependabot vulnerabilities

**Date:** 2026-07-29

## What was investigated and why
The user asked to investigate the reported Dependabot vulnerabilities and fix what
we could. Enumerated open alerts via `gh api repos/damyanp/ghui/dependabot/alerts?state=open`.

## What was found
Two open alerts:
- **#66 dompurify (npm, LOW)** — GHSA-c2j3-45gr-mqc4, a `CUSTOM_ELEMENT_HANDLING`
  hook-policy inconsistency (second-order XSS gadget). `<= 3.4.11`, patched `3.4.12`.
  Transitive via `monaco-editor@0.56.0`; the repo already had an `overrides` entry
  pinning dompurify to `^3.4.11`.
- **#1 glib (rust, MEDIUM)** — RUSTSEC-2024-0429, a NULL-deref soundness crash (not
  RCE). `glib 0.18.5` comes from the gtk-rs 0.18 stack via `tao`/`wry 0.55.1`/`tauri`
  — the Linux GTK/WebKit backend only (cfg-gated). ghui ships Windows-only (MSI/NSIS),
  so the path never runs on the shipped target. Latest `wry` (0.55.1, already in use)
  still requires gtk-rs 0.18 / `glib ^0.18`, so `glib 0.20` cannot be forced — no
  upstream fix path.

## Decisions made, and what was rejected
- **dompurify:** bumped the existing `overrides` entry `^3.4.11` → `^3.4.12`.
  Rejected committing the full `npm install` lockfile rewrite (it churned dev→devOptional
  markers and dropped bundled `@emnapi/*` entries, breaking `npm ci`); instead reverted
  the lockfile and edited only dompurify's `version`/`resolved`/`integrity` block. sha512
  integrity was computed from the tarball because the corporate proxy only returns sha1.
- **glib:** dismissed the alert as `tolerable_risk` (Linux-only backend, Windows-only
  product, no upstream fix). User chose this over leaving it open or force-bumping glib.

## Artifacts (PRs, commits, files changed)
- PR #188 — "Fix dompurify vulnerability (Dependabot #66); dismiss Linux-only glib
  alert (#1)" — merged to `main` (fast-forwarded: `9d868ed..b0c3921`).
- Changed: `app/package.json`, `app/package-lock.json` (4 lines).
- Dependabot alert #1 dismissed via API (`tolerable_risk`).
- Validation: `npm ci` ✅, `npm run build` ✅, `npm run check` ✅ (0/0), `npm test` ✅ (90 passed).

## Still open
- glib #1 remains dismissed, not fixed. Revisit when tauri/wry adopt the gtk-rs 0.20
  stack (which would pull `glib >= 0.20`).

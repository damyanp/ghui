# End of Line protocol setup

**Date:** 2026-07-29

## What was investigated and why
The user asked to set up the End of Line protocol described in `.github/end-of-line.md`.
Goal: wire a session-ending review protocol into this repo's assistant instructions and
create the tracking scaffolding.

## What was found
- Setup had not run before (`.tracking/friction.jsonl` did not exist).
- Only `.github/copilot-instructions.md` existed among candidate instruction files, so that
  is the file GitHub Copilot reads here — no ambiguity, no need to pick between several.
- The user chose the **Shared** option (commit to repo) and then reinforced that all
  findings/data must live in the repo with no locally-ignored files.
- Verified nothing was added to `.git/info/exclude` and `.gitignore` does not reference
  `.tracking` (`git check-ignore` confirmed the log is tracked).
- The repo already had many unrelated staged deletions and edits predating this work;
  these were deliberately left untouched.

## Decisions made, and what was rejected
- **Shared, not Personal** — protocol files are committed so the whole team gets them.
  Rejected the `.git/info/exclude` (personal) path per the user's explicit requirement.
- Appended the trigger block to `.github/copilot-instructions.md` (chosen instruction file).
- Used `.tracking/` (not the `.eol/` fallback) since it was free.
- Staged the commit explicitly by path — rejected `git add -A` to avoid sweeping up the
  unrelated in-progress changes.

## Artifacts (PRs, commits, files changed)
- Commit `2dacd19` "Set up End of Line protocol": `.github/copilot-instructions.md`,
  `.github/end-of-line.md`, `.tracking/friction.jsonl`.
- This End of Line run: one friction entry logged (`repeated-request`, count 1 — below
  threshold, no rule written); this session record.

## Still open
- `.tracking/sessions/` was empty until this record (git does not track empty dirs); it is
  now populated.
- Nothing else outstanding for the protocol. Unrelated pre-existing repo changes remain
  staged/unstaged and are out of scope.

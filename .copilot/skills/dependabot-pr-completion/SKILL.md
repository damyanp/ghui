---
name: "dependabot-pr-completion"
description: "Complete outstanding Dependabot PRs end-to-end: local validation, minimal fixes, and safe merge"
domain: "dependency-management"
confidence: "high"
source: "manual"
user-invocable: true
---

## Context

This skill executes the repository workflow for finishing open Dependabot PRs with minimal human intervention:
1. Find open Dependabot PRs.
2. Validate each PR locally with ecosystem-appropriate checks.
3. Apply only simple, targeted compatibility fixes when necessary.
4. Push fixes to the PR branch.
5. Merge immediately when allowed, or enable auto-merge and continue.

This repository uses grouped Dependabot updates in `.github/dependabot.yml`:
- `cargo` (workspace root)
- `npm` (`/app`)
- `github-actions` (workflow files)

Required CI checks are defined in `.github/workflows/rust.yml` (`format`, `build`, `frontend`).

## Patterns

### 1. Discover outstanding Dependabot PRs

Use `gh` and work from `main`:

```bash
gh pr list --state open --author app/dependabot --limit 50 \
  --json number,title,headRefName,baseRefName,url,mergeStateStatus
```

If there are no open PRs, stop and report completion.

### 2. Process PRs one-by-one (deterministic loop)

For each PR number `N`:

1. Inspect metadata and changed files:
```bash
gh pr view N --json title,files,mergeStateStatus,statusCheckRollup,headRefName
```

2. Check out the PR branch:
```bash
gh pr checkout N
```

3. Classify ecosystem from changed paths:
- **GitHub Actions PR**: only `.github/workflows/*` or actions-related files.
- **Cargo PR**: `Cargo.lock` and/or `Cargo.toml` changes.
- **npm PR**: `app/package.json` and/or `app/package-lock.json` changes.

### 3. Run local validation by ecosystem

Use the smallest commands that cover changed behavior.

#### GitHub Actions PR

Usually no local dependency install is needed. Confirm workflow files are coherent and then rely on CI.

#### Cargo PR

Run fast targeted Rust validation first:
```bash
cargo test -p github-graphql --verbose
```

If this fails due to dependency/API changes, make minimal fixes and re-run.

#### npm PR (`app/`)

Run clean install + frontend checks:
```bash
cd app
npm ci
npm run check
npm test
npm run build
```

If install fails due to peer conflicts introduced by the bump, apply a minimal compatibility fix (for example, pin a specific version range in `app/package.json`), update lockfile, and re-run the same checks.

### 4. Minimal fix policy

Allowed:
- Version range pin/rollback for directly bumped dependency causing install/check breakage.
- Lockfile updates corresponding to the minimal manifest change.
- Small config edits directly required by dependency migration.

Not allowed:
- Unrelated refactors.
- Broad toolchain migrations outside the PR scope.
- Force-push or history rewrite.

### 5. Commit and push fixes (execute mode default)

If local fixes were needed:

```bash
git add <changed-files>
git commit -m "fix(deps): <short compatibility fix summary>"
git push
```

Use a concise message focused on the compatibility fix.

### 6. Merge strategy

Try direct merge first:
```bash
gh pr merge N --merge --delete-branch
```

If branch is behind:
```bash
gh pr update-branch N --rebase
```

Then:
- If checks are complete and successful: merge.
- If checks are pending: enable auto-merge and move on:
```bash
gh pr merge N --merge --auto --delete-branch
```

### 7. End-state verification

After processing all PRs:
```bash
gh pr list --state open --author app/dependabot --limit 50 --json number,title,url
```

Report:
- merged PRs
- PRs set to auto-merge
- PRs blocked and why

## Failure handling

Stop and report instead of guessing when:
- A fix requires architecture-level decisions.
- Required checks repeatedly fail with non-local/infra causes.
- Merge is blocked by policy requiring human review/approval.
- Command output suggests credential/auth problems (`gh auth` issues).

When blocked, include exact PR number and shortest actionable next step.

## Example invocation prompt

```
Complete all outstanding Dependabot PRs in this repo.
Validate each PR locally, apply only minimal compatibility fixes if needed,
push fixes, and merge or enable auto-merge when checks are pending.
```

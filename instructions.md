# Developer Instructions

## Version Bumping

Every change that is pushed to `main` (and therefore triggers a release build) **must** include a version bump. This ensures the automatic update system can detect new versions.

### Files to update

When bumping the version, update all of the following in the same commit/PR:

| File | Field |
|---|---|
| `app/src-tauri/tauri.conf.json` | `"version"` |
| `app/src-tauri/Cargo.toml` | `version` |
| `ghui-app/Cargo.toml` | `version` |
| `github-graphql/Cargo.toml` | `version` |
| `ghui-util/Cargo.toml` | `version` |

All five files must have the same version string (e.g. `0.2.0`).

### Version format

Use [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`

- **PATCH** (`0.1.0` → `0.1.1`): Bug fixes, minor tweaks
- **MINOR** (`0.1.0` → `0.2.0`): New features, backward-compatible changes
- **MAJOR** (`0.1.0` → `1.0.0`): Breaking changes or significant milestones

### Code review checklist

Reviewers should verify:

- [ ] All five version fields are updated and consistent
- [ ] The version is higher than the current `main` version
- [ ] No version bump for PRs that only change documentation, CI config, or `instructions.md` itself (unless paired with other changes)

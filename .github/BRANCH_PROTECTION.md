# Branch Protection Rules

This document describes the required branch protection settings for the SheshAOS repository.

## Required Settings for `main` / `master`

### 1. Require branches to be up to date
- ✅ **Enabled**
- Ensures PRs are tested against the latest base branch

### 2. Require status checks to pass
- ✅ **Enabled**
- Required checks:
  - `lint` — formatting + clippy
  - `test` — unit + integration + doc tests
  - `build` — debug + release builds
  - `security` — cargo audit

### 3. Require conversation resolution
- ✅ **Enabled**
- All review comments must be resolved before merge

### 4. Require signed commits
- ✅ **Enabled**
- All commits must be GPG-signed

### 5. Require linear history
- ✅ **Enabled**
- Use "Squash and merge" or "Rebase and merge"
- Prevents merge commits

### 6. Include administrators
- ✅ **Enabled**
- Rules apply to everyone, including maintainers

### 7. Restrict pushes
- ✅ **Enabled**
- Only allow merges via PR
- No direct pushes to main/master

### 8. Allow force pushes
- ❌ **Disabled**
- Prevents history rewriting on protected branches

### 9. Allow deletions
- ❌ **Disabled**
- Prevents accidental branch deletion

## Merge Strategy

| Strategy | Allowed | Notes |
|----------|---------|-------|
| Merge commit | ❌ | Creates merge commits, pollutes history |
| Squash merge | ✅ | Preferred — clean linear history |
| Rebase merge | ✅ | Allowed for small PRs |

## PR Requirements

- **Title**: Must follow conventional commits (`type(scope): description`)
- **Size**: Keep under 500 lines; split larger changes
- **Checks**: All CI checks must pass
- **Reviews**: At least 1 approval from code owner
- **Draft**: Allowed, but must be marked ready before merge

## Emergency Bypass

In case of critical production fixes:
1. Get approval from 2 maintainers
2. Document the emergency in the PR description
3. Revert the bypassed PR and re-open as normal PR after fix

## Enforcement

CI enforces:
- PR title convention
- No merge conflicts
- Test count threshold (≥100 tests)
- Architecture artifact presence
- No orphaned `src/` directory

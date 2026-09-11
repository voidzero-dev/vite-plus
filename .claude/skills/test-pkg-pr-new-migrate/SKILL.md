---
name: test-pkg-pr-new-migrate
description: Verify a preview (registry bridge) build of vite-plus against a real project before release — run `vp migrate` from the preview commit against a local project, deps resolved through the registry bridge. Use when asked to verify/e2e-test a preview or pkg-pr-new build against a project, "test PR #<N> on <project>", or check a prerelease against a repo.
allowed-tools: Bash, Read
---

# Verify a preview build against one project

Installs an isolated global `vp` built from a registry bridge commit build and runs `vp migrate` on a specified local project. The global CLI and the migrated project both pin `vite-plus`/`vite` to the clearly-defined `0.0.0-commit.<sha>` build. `vp migrate` itself writes the bridge registry into the project's `.npmrc` (or `.yarnrc.yml` for Yarn Berry) so the deps resolve, during this run and in the project's own CI; this script only force-stages that file past `.gitignore`.

Required inputs: a `<PR-or-SHA>` (the build to verify) and a `<project-path>`. If either is missing from the request, **ask the user** for it — never guess the PR/SHA, as the build under test is the user's choice.

```bash
.github/scripts/test-pkg-pr-new-migrate.sh <PR-or-SHA> <project-path> [migrate-options...]
# e.g.
.github/scripts/test-pkg-pr-new-migrate.sh 1891 /path/to/npmx.dev --no-interactive
```

- First arg is a PR number or commit SHA; the script resolves the immutable commit via the bridge `x-commit-key` header and verifies the bridge serves it (the preview publish workflow, triggered by the `preview-build` label, registers each commit).
- Never touches `~/.vite-plus`; clears only the workspace ROOT lockfile + `node_modules` before migrating; refuses a dirty worktree unless `ALLOW_DIRTY=1`; prints the project's `git status`/`diff` at the end — inspect that to confirm the migration result.

**The build under test must include the "migrate writes the bridge registry" feature** (this session's work / current branch head onward). The harness no longer writes the registry itself — it relies on `vp migrate` doing it. Testing an older build with this harness would leave the project with no bridge registry, so its deps resolve from npmjs (`ERR_PNPM_NO_MATCHING_VERSION` on the `0.0.0-commit.<sha>` version). Always verify a fresh build of the branch, not a stale published commit.

The script prints the resolved versions at the end, querying one package at a time (npm/yarn/bun `why` only accept a single package; `-r` is pnpm-only). Each must resolve to exactly ONE version: `vite-plus` and `vite` at the expected `0.0.0-commit.<sha>` (vite via the `@voidzero-dev/vite-plus-core` alias), `vitest` at the bundled upstream version. Multiple versions, or a stale/wrong version, means the migration or install is broken. To re-check by hand:

```bash
cd <project-path>
# pnpm: one call, recursive across workspaces
vp why -r vite-plus vite vitest
# npm / yarn / bun: one package per call, no -r
vp why vite-plus && vp why vite && vp why vitest
```

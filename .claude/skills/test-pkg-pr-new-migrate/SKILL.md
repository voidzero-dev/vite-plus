---
name: test-pkg-pr-new-migrate
description: Verify a pkg.pr.new build of vite-plus against a real project before release — run `vp migrate` from the pkg.pr.new commit against a local project, deps resolved through the registry bridge. Use when asked to verify/e2e-test a pkg.pr.new build against a project, "test PR #<N> on <project>", or check a prerelease against a repo.
allowed-tools: Bash, Read
---

# Verify a pkg.pr.new build against one project

Installs an isolated global `vp` from a pkg.pr.new commit and runs `vp migrate` on a specified local project. The result pins `vite-plus`/`vite` to `0.0.0-commit.<sha>` resolved through the registry bridge, persisted into the project's `.npmrc` (and `.yarnrc.yml` for Yarn Berry) so the project's own CI installs the build too.

Required inputs: a `<PR-or-SHA>` (the pkg.pr.new build to verify) and a `<project-path>`. If either is missing from the request, **ask the user** for it — never guess the PR/SHA, as the build under test is the user's choice.

```bash
.github/scripts/test-pkg-pr-new-migrate.sh <PR-or-SHA> <project-path> [migrate-options...]
# e.g.
.github/scripts/test-pkg-pr-new-migrate.sh 1891 /path/to/npmx.dev --no-interactive
```

- First arg is a PR number or commit SHA; the script resolves the immutable commit and verifies the bridge serves it (the pkg.pr.new publish workflow registers each commit).
- Never touches `~/.vite-plus`; refuses a dirty worktree unless `ALLOW_DIRTY=1`; prints the project's `git status`/`diff` at the end — inspect that to confirm the migration result.

Then confirm the resolved versions (`-r` across workspaces for monorepos):

```bash
cd <project-path>
vp why -r vite-plus vite vitest
```

Each must resolve to exactly ONE version: `vite-plus` and `vite` at the expected `0.0.0-commit.<sha>` (vite via the `@voidzero-dev/vite-plus-core` alias), `vitest` at the bundled upstream version. Multiple versions, or a stale/wrong version, means the migration or install is broken.

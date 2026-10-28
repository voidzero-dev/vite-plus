# Reproduce

Determine whether the reported Vite+ bug reproduces on the current checkout.

**Always write `report.md` in `triageDir` before returning. Do not diagnose or fix in this step.**

## 1. Read the Report

Read the title, body, environment, reproduction, expected behavior, actual behavior, and all comments in `issueDetails`. Later comments can add details or invalidate an early exit.

## 2. Check Early Exits

Return `skipped: true` with one of these reasons only when it clearly applies:

- `not-actionable`: not a Vite+ bug report, such as a feature request, question, discussion, or support request.
- `missing-details`: no usable reproduction or no expected behavior.
- `unsupported-version`: requires a version line that current Vite+ no longer supports and cannot be tested on current `main`.
- `host-specific`: requires Windows, macOS, a specific CPU architecture, a shell unavailable on Ubuntu, a hosted service, or credentials.
- `unsupported-runtime`: requires a JavaScript runtime or package-manager version that cannot be installed on the runner.
- `maintainer-override`: a `MEMBER`, `COLLABORATOR`, or `OWNER` says not to auto-triage.

Do not skip only because the reporter used an older Vite+ release. Test current `main`. If it does not reproduce, record that it may already be fixed and return `reproducible: false, skipped: false`.

## 3. Prepare

```bash
VITE_PLUS_REPO="$(git rev-parse --show-toplevel)"
TRIAGE_DIR="/tmp/vite-plus-triage/gh-<issue-number>"
mkdir -p "$TRIAGE_DIR"
vp --version
node --version
pnpm --version
```

Create reproductions under `$TRIAGE_DIR/repro`.

- GitHub repository: `git clone --depth 1 <url> "$TRIAGE_DIR/repro"`.
- StackBlitz project: `npx stackblitz-clone@latest <url> "$TRIAGE_DIR/repro"`.
- Manual steps: create the smallest project that matches the report.

Before installing a reproduction, inspect its package scripts. Prefer `--ignore-scripts` unless lifecycle scripts are part of the bug.

## 4. Use the Local Build

- Rust global CLI, package-manager, and runtime commands: run the `vp` on `PATH`.
- JS-backed project commands: if the project has published `vite-plus`, run `pnpm link "$VITE_PLUS_REPO/packages/cli"` in the reproduction and confirm `node_modules/vite-plus` points to the checkout.
- `vp create` or `vp migrate`: wrap the command with `node "$VITE_PLUS_REPO/packages/tools/src/local-npm-registry.ts" --pack -- <command>`.
- Global state or shell setup: use a fresh `VP_HOME`, set `VP_SELF_SETUP_NO_MODIFY_PATH=1`, install with `(cd "$VITE_PLUS_REPO" && pnpm install-global-cli)`, and call `$VP_HOME/bin/vp`. Never run `vp implode` or `vp upgrade` against the default installation.

## 5. Reproduce

1. Run the reported trigger and capture the exact command, exit code, stdout, stderr, and stack trace.
2. Remove or change the trigger and run a control case to rule out an unrelated failure.
3. Prefer build, test, and command output over long-running dev servers. Use timeouts and stop servers you start.
4. Stop after one clear reproduction.

Return `reproducible: true` only when the reported incorrect behavior occurs with the current checkout.

## 6. Write `report.md`

Include:

- Issue summary and expected versus actual behavior.
- Vite+, Node.js, package-manager, OS, and relevant tool versions.
- Reproduction path and every command attempted.
- Exact errors and the control result.
- Result: reproduced, not reproduced, or skipped, with the reason.
- Observations useful for diagnosis, without proposing a fix.

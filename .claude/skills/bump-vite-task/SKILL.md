---
name: bump-vite-task
description: Bump vite-task git dependency to the latest main commit. Use when you need to update the vite-task git-dependency crates (vite_task, fspy, pty_terminal_test, and friends; the authoritative set lives in Cargo.toml) in vite-plus.
allowed-tools: Read, Grep, Glob, Edit, Bash, Agent, WebFetch
---

# Bump vite-task to Latest Main

Update the vite-task git dependency in `Cargo.toml` to the latest commit on the vite-task main branch, fix any breaking changes, and create a PR.

## Steps

### 1. Get current and target commits

- Read `Cargo.toml` and find the current `rev = "..."` for any vite-task git dependency; enumerate the full set with `grep 'voidzero-dev/vite-task' Cargo.toml` instead of trusting a hardcoded list. They all share the same revision.
- Get the latest commit hash on vite-task's main branch:
  ```bash
  git ls-remote https://github.com/voidzero-dev/vite-task.git refs/heads/main
  ```

### 2. Update Cargo.toml

- Replace **all** occurrences of the old commit hash with the new one in `Cargo.toml`. The set of crates sharing the vite-task revision changes over time; the grep from step 1 is the authoritative list, so update every entry it returns.
- The commented `[patch."https://github.com/voidzero-dev/vite-task.git"]` section near the bottom of `Cargo.toml` mirrors the same crates for local vite-task development; keep it in sync when crates are added or removed (a plain rev bump does not touch it).

### 3. Ensure upstream dependencies are cloned

- `cargo check` requires the `./rolldown` and `./vite` directories to exist (many workspace path dependencies point to `./rolldown/crates/...`).
- Locally, clone them using the commit hashes from `packages/tools/.upstream-versions.json`.
- CI handles this automatically via the `.github/actions/clone` action.

### 4. Verify compilation

- Run `cargo check` to ensure the new vite-task compiles without errors.
- If there are compilation errors, these are **breaking changes** from vite-task. Fix them in the vite-plus codebase (the consuming side), not in vite-task.
- Common breaking changes include: renamed functions/methods, changed function signatures, new required fields in structs, removed public APIs.

### 5. Run tests

- Run `cargo test -p vite_command -p vite_error -p vite_install -p vite_js_runtime -p vite_migration -p vite_shared -p vite_static_config -p vite-plus-cli -p vite_global_cli` to run the vite-plus crate tests.
- Note: Some tests require network access (e.g., `vite_install::package_manager` tests, `vite_global_cli::commands::env` tests). These may fail in sandboxed environments. Verify they also fail on the main branch before dismissing them.
- Note: `cargo test -p vite_task` will NOT work because vite_task is a git dependency, not a workspace member.
- The PTY snapshot suite (`crates/vite_cli_snapshots`) is excluded from `just test`; it is covered in step 6.

### 6. Update snapshot tests

vite-task changes often affect CLI output, which means snapshot tests need updating. Common output changes:

- **Status icons**: e.g., cache hit/miss indicators may change
- **New CLI options**: e.g., new flags added to `vp run` that show up in help output
- **Cache behavior messages**: e.g., new summary lines about cache status
- **Task output formatting**: e.g., step numbering, separator lines

**PTY snapshot suite (`crates/vite_cli_snapshots`):**

- A bump can break it two ways: runner compilation (it consumes vite-task's `pty_terminal_test`, `pty_terminal_test_client`, and `snapshot_test` crates directly, so their API changes surface here; fix in the runner) and recorded CLI output.
- Update output locally with real assertions: `UPDATE_SNAPSHOTS=1 just snapshot-test`, then review the `.md` diffs like code. Without a built `packages/cli/dist`, run the global flavor only: `VP_SNAP_SKIP_FLAVORS=local UPDATE_SNAPSHOTS=1 just snapshot-test`.
- Windows runs in the `CLI snapshot test (Windows)` CI job via a cross-compiled nextest archive; snapshots are OS-shared, so a Windows-only diff there means a redaction gap, not a re-record.
- Reference: `crates/vite_cli_snapshots/tests/cli_snapshots/README.md`.

### 7. Review changelog and update docs

- Fetch the vite-task `CHANGELOG.md` diff between old and new commits to see what changed:
  ```
  https://raw.githubusercontent.com/voidzero-dev/vite-task/<new-hash>/CHANGELOG.md
  ```
- Review each changelog entry and determine if it affects user-facing behavior: new CLI options, changed defaults, new config fields, removed features, etc.
- The changelog contains links to the corresponding vite-task PRs. For complex changes, check the PR description and code diff (especially any docs changes in the PR) to understand the full scope of the change.
- If user-facing changes are found, update the relevant docs in `docs/` (e.g., `docs/guide/`, `docs/config/`).
- Common doc updates include:
  - **New CLI flags/options**: Update the relevant config doc (e.g., `docs/config/run.md`, `docs/config/build.md`)
  - **New features or commands**: Add or update the relevant guide page (e.g., `docs/guide/cache.md`)
  - **Changed defaults or behavior**: Update any docs that describe the old behavior
  - **Removed/deprecated options**: Remove or mark as deprecated in the relevant docs
- If no user-facing changes are found, skip this step.

### 8. Create the PR

- Commit message: `chore: bump vite-task to <short-hash>`
- PR title: `chore: bump vite-task to <short-hash>`
- PR body: Link to vite-task CHANGELOG.md diff between old and new commits:
  ```
  https://github.com/voidzero-dev/vite-task/compare/<old-hash>...<new-hash>#diff-06572a96a58dc510037d5efa622f9bec8519bc1beab13c9f251e97e657a9d4ed
  ```

### 9. Verify CI

After creating the PR, automatically watch CI without asking the user first. Ensure the `done` check passes. Key checks to monitor:

- **Lint**: Clippy and format checks
- **Test** (Linux, Mac, Windows): Rust unit tests
- **CLI snapshot test** (Linux, Mac, Windows): the PTY snapshot suite - most likely to fail on a vite-task bump (runner compiles against vite-task crates AND asserts CLI output)
- **Run task**: Task runner integration tests
- **Cargo Deny**: License/advisory checks (may have pre-existing failures unrelated to bump)

The only **required** status check for merging is `done`, which aggregates the other checks (excluding Cargo Deny).

## Notes

- Building the full CLI locally (`pnpm bootstrap-cli`) requires the rolldown Node.js package to be built first, which is complex. Prefer the global-only snapshot command above or CI when no local CLI build is available.
- `Cargo.lock` is automatically updated by cargo when you change the revision in `Cargo.toml`.

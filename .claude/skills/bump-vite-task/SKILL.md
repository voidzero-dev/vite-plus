---
name: bump-vite-task
description: Bump vite-task git dependency to the latest main commit. Use when you need to update the vite-task crates (fspy, vite_glob, vite_path, vite_str, vite_task, vite_workspace) in vite-plus.
allowed-tools: Read, Grep, Glob, Edit, Bash, Agent, WebFetch
---

# Bump vite-task to Latest Main

Update the vite-task git dependency in `Cargo.toml` to the latest commit on the vite-task main branch, fix any breaking changes, and create a PR.

## Steps

### 1. Get current and target commits

- Read `Cargo.toml` and find the current `rev = "..."` for any vite-task git dependency (e.g., `vite_task`, `vite_path`, `fspy`, `vite_glob`, `vite_str`, `vite_workspace`). They all share the same revision.
- Get the latest commit hash on vite-task's main branch:
  ```bash
  git ls-remote https://github.com/voidzero-dev/vite-task.git refs/heads/main
  ```

### 2. Update Cargo.toml

- Replace **all** occurrences of the old commit hash with the new one in `Cargo.toml`. There are 6 crate entries that reference the same vite-task revision: `fspy`, `vite_glob`, `vite_path`, `vite_str`, `vite_task`, `vite_workspace`.

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

### 6. Update snap tests

vite-task changes often affect CLI output, which means snap tests need updating. Common output changes:

- **Status icons**: e.g., cache hit/miss indicators may change
- **New CLI options**: e.g., new flags added to `vp run` that show up in help output
- **Cache behavior messages**: e.g., new summary lines about cache status
- **Task output formatting**: e.g., step numbering, separator lines

To update snap tests:

1. Push your changes and let CI run the snap tests.
2. CI will show the diff in the E2E test logs if snap tests fail.
3. Extract the diff from CI logs and apply it locally.
4. Check all three platforms (Linux, Mac, Windows) since they may have slightly different snap test coverage.
5. Watch for trailing newline issues - ensure snap files end consistently.

Snap test files are at `packages/cli/snap-tests/*/snap.txt` and `packages/cli/snap-tests-global/*/snap.txt`.

### 7. Create the PR

- Commit message: `chore: bump vite-task to <short-hash>`
- PR title: `chore: bump vite-task to <short-hash>`
- PR body: Link to vite-task CHANGELOG.md diff between old and new commits:
  ```
  https://github.com/voidzero-dev/vite-task/compare/<old-hash>...<new-hash>#diff-06572a96a58dc510037d5efa622f9bec8519bc1beab13c9f251e97e657a9d4ed
  ```

### 8. Verify CI

Wait for CI and ensure the `done` check passes. Key checks to monitor:

- **Lint**: Clippy and format checks
- **Test** (Linux, Mac, Windows): Rust unit tests
- **CLI E2E test** (Linux, Mac, Windows): Snap tests - most likely to fail on a vite-task bump
- **Run task**: Task runner integration tests
- **Cargo Deny**: License/advisory checks (may have pre-existing failures unrelated to bump)

The only **required** status check for merging is `done`, which aggregates the other checks (excluding Cargo Deny).

## Notes

- Building the full CLI locally (`pnpm bootstrap-cli`) requires the rolldown Node.js package to be built first, which is complex. Prefer relying on CI for snap test generation.
- `Cargo.lock` is automatically updated by cargo when you change the revision in `Cargo.toml`.

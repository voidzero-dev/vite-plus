# Fix

Implement and verify a minimal Vite+ fix when the earlier stages support one.

**Read and append to `report.md` before returning. Do not commit or push.**

## 1. Decide Whether to Fix

Return `fixed: false` without changing the repository when any of these apply:

- The bug was not reproduced or was skipped.
- Diagnosis confidence is `low` or the root cause is unclear.
- The verdict is `intended-behavior`.
- The root cause is only in an upstream clone.
- The fix needs a product decision, dependency upgrade, release, or broad refactor.

## 2. Implement

- Change only Vite+ source and the tests needed for this bug.
- Follow `AGENTS.md`, `.clippy.toml`, and nearby code.
- Do not edit `rolldown/`, `vite/`, `oxc-node/`, `.github/`, `.agents/`, release configuration, credentials, or lockfiles.
- Keep public test imports on `vite-plus/test*`.
- Use the repository helpers for Rust directory resolution and user-facing output.

## 3. Add a Regression Test

Put the test beside related coverage.

- TypeScript: use nearby `__tests__/*.spec.ts` or `*.spec.ts` files.
- Rust: use the affected crate's unit or integration tests.
- CLI output or interaction: read `crates/vp_cli_snapshots/tests/cli_snapshots/README.md` and add a fixture under `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/`.

Confirm that the test fails without the fix when practical and passes with it.

## 4. Validate

1. Rebuild the changed layer.
2. Re-run the original reproduction and the control case.
3. Run focused tests first, then the relevant checks from `AGENTS.md` when they fit in the job time.

Common checks:

- TypeScript: focused `vp test <file>`, then `vp check`.
- Rust: focused `cargo test -p <crate> <filter>`, then `cargo fmt --all --check` and the relevant `just check`, `just test`, or `just lint` command.
- CLI snapshots: `just snapshot-test <filter>`. Use `UPDATE_SNAPSHOTS=1` only for an intended change, then rerun without it and review the snapshot diff.

Return `fixed: true` only when the original reproduction passes, the regression test passes, and relevant checks pass.

## 5. Clean Up

Remove debug code and unrelated generated changes. Move reports, reproduction projects, reference checkouts, logs, scratch scripts, build artifacts, and other triage data into `triageDir`, or delete temporary files that are no longer needed. Do not commit triage data.

The action stages every new or changed file in the checkout. Review `git status --short --untracked-files=all` before returning; it must show only intended source and regression-test changes.

## 6. Append to `report.md`

Include the changed files, rationale, full `git diff`, regression test, validation commands and results, skipped checks with reasons, and alternatives considered.

When fixed, return a conventional commit message such as `fix(cli): preserve config path in vp check`. Otherwise return `commitMessage: null`.

# Diagnose

Find the root cause of a reproduced Vite+ bug.

**Read and append to `report.md` before returning. Do not fix the bug in this step.**

If the reproduce step did not reproduce the bug or skipped it, append `DIAGNOSIS SKIPPED: No reproduction` and return `confidence: null`.

## 1. Locate the Code Path

Use the reproduction, stack traces, and `AGENTS.md` to choose the relevant layer.

- Global CLI routing: `crates/vp_global_cli/src/main.rs` and `crates/vp_global_cli/src/cli.rs`.
- Local JS and NAPI commands: `packages/cli/src/bin.ts`, `packages/cli/binding/src/lib.rs`, and `packages/cli/binding/src/cli/mod.rs`.
- Package managers and runtimes: `crates/vp_pm_cli/` and `crates/vp_js_runtime/`.
- Windows shims: `crates/vp_trampoline/`.
- Config loading: `crates/vp_static_config/README.md` and `packages/cli/src/resolve-vite-config.ts`.
- Project creation: `packages/cli/src/create/`.
- Migration: `packages/cli/src/migration/`, `packages/cli/src/migration/migrator/README.md`, and `docs/guide/migrate-rules.md`.
- Test integration and bundled exports: `packages/cli/src/resolve-test.ts`, `packages/cli/build.ts`, `packages/cli/BUNDLING.md`, and `packages/core/BUNDLING.md`.
- Directories and user output: `crates/vp_shared/src/dirs.rs` and `crates/vp_shared/src/output.rs`.

The `rolldown/`, `vite/`, and `oxc-node/` directories are upstream clones. If the root cause is upstream, document it and do not plan a patch to those clones.

## 2. Trace the Failure

Read the code before adding instrumentation. If instrumentation is needed, keep it temporary and rebuild the affected layer.

- TypeScript in `packages/cli`: `pnpm -F vite-plus build-ts`.
- NAPI binding: `pnpm -F vite-plus build`.
- Global Rust CLI: `cargo build -p vp_global_cli --release && pnpm install-global-cli`.

Re-run the smallest reproduction. Revert all instrumentation and confirm with `git diff` before returning.

## 3. Assign Confidence

- `high`: the failing logic and its effect are proven by code inspection and observed behavior.
- `medium`: the likely failing logic is clear, but some behavior is inferred.
- `low`: only related areas are known.

## 4. Append to `report.md`

Include the affected files and lines, the incorrect logic, why it causes the observed behavior, a suggested fix approach, edge cases, regression evidence if found, and confidence.

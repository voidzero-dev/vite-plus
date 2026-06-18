# AI Agent Guidelines for Vite-Plus

This document helps AI coding assistants work on the **vite-plus repository**. It is repo-specific: `CLAUDE.md` points here for compatibility, while `packages/cli/AGENTS.md` is the generated guidance shipped to projects that use Vite+.

Use this file to orient quickly, choose the right code area, and pick validation that matches the change. Prefer canonical source files and docs over duplicating details here.

## Project Overview

Vite+ is **the unified toolchain for the web** behind the `vp` CLI. It combines Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, Vite Task, runtime management, package-manager workflows, project creation/migration, and monorepo task caching.

### Key Technologies

- **TypeScript / Node ESM**: CLI entrypoints, create/migrate/config/staged commands, docs tooling, and package exports.
- **Rust 2024**: global `vp` binary, local CLI binding, package-manager/runtime helpers, config extraction, shared utilities.
- **NAPI**: bridges the TypeScript CLI package to Rust command implementations under `packages/cli/binding/`.
- **pnpm workspaces**: local packages under `packages/*`.
- **Vite+ config**: `vite.config.ts` configures lint, format, test, and run tasks for this repo.

## Architecture

High-signal repo map:

```
vite-plus/
├── packages/cli/              # Published vite-plus package, JS CLI, docs, templates, snap tests, NAPI binding
│   ├── src/bin.ts             # JS entrypoint and local CLI dispatch
│   ├── src/utils/agent.ts     # Generated agent-file marker/update logic
│   ├── binding/               # Rust NAPI binding used by the local CLI
│   ├── snap-tests/            # Local CLI output snapshots
│   └── snap-tests-global/     # Global CLI output snapshots
├── packages/core/             # @voidzero-dev/vite-plus-core bundled toolchain surfaces
├── packages/test/             # @voidzero-dev/vite-plus-test Vitest-compatible exports
├── packages/prompts/          # Prompt UI/helpers package
├── packages/tools/            # Repo tooling
├── crates/vite_global_cli/    # Standalone global vp binary and top-level command routing
├── crates/vite_pm_cli/        # Package-manager command surface
├── crates/vite_install/       # Package-manager detection/install behavior
├── crates/vite_js_runtime/    # Managed Node.js runtime support
├── crates/vite_static_config/ # Static extraction of vite.config.* data
└── crates/vite_shared/        # Shared Rust env config, tracing, output, utilities
```

## Where to Start

- **JS-backed CLI behavior**: start at `packages/cli/src/bin.ts` and nearby `packages/cli/src/**` files.
- **Local CLI / NAPI-backed behavior**: start at `packages/cli/binding/src/lib.rs` and `packages/cli/binding/src/cli/mod.rs`.
- **Global `vp` routing, aliases, and runtime bootstrap**: start at `crates/vite_global_cli/src/main.rs` and `crates/vite_global_cli/src/cli.rs`.
- **Package-manager commands**: start at `crates/vite_pm_cli/` and `crates/vite_install/`.
- **Managed Node runtime / shims**: start at `crates/vite_js_runtime/`.
- **Static `vite.config.ts` extraction**: start at `crates/vite_static_config/README.md` and `packages/cli/src/resolve-vite-config.ts`.
- **Bundled toolchain surfaces**: start with `packages/core/BUNDLING.md`, `packages/cli/BUNDLING.md`, and `packages/test/BUNDLING.md`.
- **Generated project agent guidance**: `packages/cli/AGENTS.md` and `packages/cli/src/utils/agent.ts`; do not edit these when the task is only to improve root repo guidance.
- **Product/repo docs**: root contributor docs live at the repo root and the VitePress site under `docs/` (`docs/guide/`, `docs/config/`); generated agent guidance is separate.
- **CLI output behavior**: inspect the relevant code plus `packages/cli/snap-tests/` or `packages/cli/snap-tests-global/`.

## Command and Config Model

`vp` has built-in toolchain commands and a separate task runner:

```bash
vp dev       # Vite dev server
vp build     # Vite + Rolldown build
vp test      # Bundled Vitest
vp lint      # Oxlint
vp fmt       # Oxfmt
vp check     # Format/lint/type-check workflow
vp pack      # Library/app packaging

vp run build # Run a package.json script or configured run task
vpr build    # Shorthand for vp run build
```

Important distinctions:

- `vp test` is a built-in command. `vp run test` runs a `package.json` script or `vite.config.ts` run task named `test`.
- Existing `package.json` scripts are first-class `vp run <script>` targets and are not cached by default.
- Define `run.tasks` in `vite.config.ts` when a task needs explicit command config, default caching, dependencies, input tracking, or environment tracking.
- A task name can come from `package.json` or `vite.config.ts`, but not both.
- Do not introduce `vite-task.json`; current Vite+ task configuration lives under `run` in `vite.config.ts`.
- Do not run `cargo test -p vite_task` in this repo; Vite Task crates are git dependencies, not local workspace members.

Reference: `docs/guide/run.md` and `docs/config/run.md`.

## Development Workflow

### Initial setup

```bash
just init
```

### Build and install from source

```bash
just build          # Release build for Vite+
pnpm bootstrap-cli  # Build packages, compile vp/NAPI, and install the global CLI
vp --version
```

Use `pnpm bootstrap-cli` when you need to validate the installed global CLI at `~/.vite-plus`.

### Routine validation

Choose checks by change type:

| Change type                   | Useful validation                                                       |
| ----------------------------- | ----------------------------------------------------------------------- |
| Docs-only / agent-guide edits | Check referenced paths and commands; run `git diff --check -- <files>`  |
| TypeScript or JS CLI behavior | `vp check`, `pnpm test:unit`, plus focused package tests when available |
| Rust CLI/crate behavior       | `just check`, `just test`, `just lint`                                  |
| CLI output or command UX      | Relevant snap tests, then inspect `git diff` for `snap.txt` changes     |
| Global CLI behavior           | `pnpm bootstrap-cli`, `vp --version`, then relevant global snap tests   |
| Release/build behavior        | `just build`                                                            |
| Pre-merge/full validation     | `pnpm bootstrap-cli && pnpm test && git status`                         |

Use `vp check --fix` only when you intentionally want formatting or lint fixes applied.

### Snap tests

Snap tests live in `packages/cli/snap-tests/` and `packages/cli/snap-tests-global/`.

```bash
pnpm -F vite-plus snap-test
pnpm -F vite-plus snap-test-local
pnpm -F vite-plus snap-test-local <name-filter>
pnpm -F vite-plus snap-test-global
pnpm -F vite-plus snap-test-global <name-filter>
```

Snap tests regenerate `snap.txt` and can exit successfully even when output changed. Always inspect `git diff` afterward. Ensure fixture inputs are committed or created by the test setup, not accidentally supplied by ignored local files.

## Code Conventions

### Rust

Reference these files instead of duplicating rules here:

- `.clippy.toml` — custom lint restrictions and disallowed Rust APIs/macros.
- `crates/vite_shared/src/output.rs` — shared user-facing output helpers.
- `crates/vite_shared/src/env_config.rs` — test-scoped environment configuration helpers.
- `crates/vite_command/src/lib.rs` and `crates/vite_global_cli/src/cli.rs` — examples of `vite_path` usage in local Rust code.

Prefer shared output helpers for user-facing messages and match nearby command style. New Rust code should satisfy the custom clippy restrictions.

### TypeScript

Reference these files instead of duplicating rules here:

- `packages/cli/src/utils/terminal.ts` — shared user-facing terminal output helpers.
- `tsconfig.json` — TypeScript compiler settings.
- `vite.config.ts` — repository lint, format, test, and task configuration.
- `packages/cli/src/utils/agent.ts` and `packages/cli/AGENTS.md` — generated/migrated project agent guidance.

## Testing Strategy

Use the validation matrix above as the source of truth. For behavior-bearing changes, find the nearest existing tests before editing and add or update coverage in the same area. For CLI output changes, pair focused tests with snap-test diff review. For documentation-only changes, verify referenced paths, commands, and links instead of running unrelated suites.

## Common Pitfalls

- **Treating Vite+ as only Vite Task**: Vite Task is integrated, but this repo spans CLI, runtime, package management, bundled packages, create/migrate, docs, and upstream integration.
- **Looking for local `crates/vite_task`**: it does not exist here. Check `Cargo.toml` for git dependency wiring.
- **Confusing built-ins with scripts**: `vp test` and `vp run test` can do different things.
- **Trusting snap-test exit status alone**: always inspect snapshot diffs.

## Debugging

- Use `vp --version` to see bundled tool versions before researching tool behavior.
- Use `vp help` and `vp <command> --help` to inspect command surfaces.
- For command routing bugs, compare the global path (`crates/vite_global_cli/`) with the local/NAPI path (`packages/cli/src/bin.ts`, `packages/cli/binding/`).
- For config-loading bugs, compare the static extractor (`crates/vite_static_config/`) with the JS resolver fallback (`packages/cli/src/resolve-vite-config.ts`).
- For package-manager behavior, inspect `crates/vite_pm_cli/`, `crates/vite_install/`, and related snap tests.
- For `vp check`, lint, format, or type-check behavior, validate end-to-end because bundled-tool routing can hide silent config drift.

## AI Assistant Tips

- Identify the ownership layer before editing: JS CLI, Rust global CLI, NAPI/local CLI, package manager, runtime, config, docs, or bundled packages.
- Prefer source references over copied explanations when adding guidance; this file should help agents navigate, not replace the docs.
- Keep diffs scoped to the requested area and leave unrelated tracked or untracked files alone.

## References

- Product overview: `README.md`
- Contributor workflow: `CONTRIBUTING.md`
- Root scripts: `package.json`
- Repo config: `vite.config.ts`
- Vite+ guide: `docs/guide/index.md`
- Run guide: `docs/guide/run.md`
- Run config: `docs/config/run.md`
- CLI package architecture: `packages/cli/BUNDLING.md`
- Core package architecture: `packages/core/BUNDLING.md`
- Test package architecture: `packages/test/BUNDLING.md`
- Generated agent guidance: `packages/cli/AGENTS.md`, `packages/cli/src/utils/agent.ts`

# Vite+

Vite+ is the web toolchain behind `vp`: Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, Vite Task, runtime and package-manager workflows, and project creation/migration.

This guide applies to this repository; `CLAUDE.md` points here. For changes to the guidance shipped to user projects, use `packages/cli/AGENTS.md` and `packages/cli/src/utils/agent.ts`. Leave those files unchanged for root-guide tasks.

## Context by Task

- **CLI routing**: `packages/cli/src/bin.ts` dispatches JS commands; `packages/cli/binding/src/lib.rs` and `packages/cli/binding/src/cli/mod.rs` handle local NAPI commands. Global routing starts at `crates/vp_global_cli/src/main.rs` and `crates/vp_global_cli/src/cli.rs`. Compare global and local paths for routing bugs.
- **Package managers and runtimes**: use `crates/vp_pm_cli/` and `crates/vp_js_runtime/`. The Windows shim trampoline lives in `crates/vp_trampoline/`, outside the Cargo workspace.
- **Config loading**: compare the [static extractor](crates/vp_static_config/README.md) with the JS fallback in `packages/cli/src/resolve-vite-config.ts`.
- **Migration**: follow the [migrator instructions](packages/cli/src/migration/migrator/README.md) when changing `vp migrate`; use [migration rules](docs/guide/migrate-rules.md) for expected behavior.
- **Bundled tools and exports**: use [CLI bundling](packages/cli/BUNDLING.md) and [core bundling](packages/core/BUNDLING.md). Test resolution starts at `packages/cli/src/resolve-test.ts`; `packages/cli/build.ts` generates test API shims.
- **Testing a local build in a project**: follow [CONTRIBUTING.md](CONTRIBUTING.md) for linking and installation. For `vp migrate` / `vp create`, use the local registry in `packages/tools/src/local-npm-registry.ts`, which also serves PTY fixtures and ecosystem tests.
- **Product documentation**: start at `README.md` and the VitePress site in `docs/guide/` and `docs/config/`.
- **Setup and build commands**: use `CONTRIBUTING.md`, `justfile`, and `package.json`. Start with `just init` for initial setup. Repository lint, format, and test configuration lives in `vite.config.ts`; TypeScript settings live in `tsconfig.json`.

## Repository Constraints

- Distinguish built-ins from tasks: `vp test` executes upstream Vitest; `vp run test` executes a script or configured task named `test`. `vpr` is shorthand for `vp run`. Inspect command surfaces with `vp help` / `vp <command> --help`, and bundled versions with `vp --version`.
- Keep task configuration under `run` in `vite.config.ts`; do not introduce `vite-task.json`. Existing `package.json` scripts are `vp run` targets without caching by default. Use `run.tasks` for explicit command configuration, default caching, dependencies, or input/environment tracking. A task name must occur in only one of these files. See the [run guide](docs/guide/run.md) and [run configuration](docs/config/run.md).
- Keep public test imports on `vite-plus/test*`, which wraps upstream `vitest` and `@vitest/browser*`. Do not recreate `packages/test` or `@voidzero-dev/vite-plus-test`.
- Vite Task crates are git dependencies in `Cargo.toml`; there is no local `crates/vt`. Do not run `cargo test -p vt` here.
- Use `vp_shared::VpDirs` for Vite+ directory roots. See `crates/vp_shared/src/dirs.rs` and `crates/vp_shared/src/dirs/resolution.rs`. Call sites must not construct category paths or read `VP_HOME` or `XDG_*` directly.
- Follow [`.clippy.toml`](.clippy.toml) for Rust restrictions and replacements. Use `crates/vp_shared/src/output.rs` for user-facing output and `crates/vp_shared/src/env_config.rs` for test-scoped environment configuration. See `crates/vp_command/src/lib.rs` for `vt_path` usage.
- For TypeScript CLI output, use `packages/cli/src/utils/terminal.ts` and match the surrounding command style.
- Keep changes scoped to the task and leave unrelated tracked and untracked files alone. Prefer source references over duplicated instructions in this guide.

## Checks and Tests

For behavior changes, find nearby tests before editing and add or update coverage in the same area. Choose checks for the changed layer:

| Change                         | Validation                                                                                                                         |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| Documentation / agent guidance | Verify referenced paths, commands, and links; `git diff --check -- <files>`                                                        |
| TypeScript / JS CLI            | `vp check`, `pnpm test:unit`, and focused package tests                                                                            |
| Rust CLI / crates              | `just check`, `just test`, `just lint`                                                                                             |
| CLI output / interactions      | Focused tests and `just snapshot-test <filter>`; review snapshot diffs                                                             |
| Global CLI                     | `pnpm bootstrap-cli` and `vp --version` for installed end-to-end checks; `just snapshot-test-global <filter>` for global snapshots |
| Release / build                | `just build`                                                                                                                       |
| Full pre-merge check           | `pnpm bootstrap-cli && pnpm test && git status`                                                                                    |

Validate `vp check`, lint, format, and type-check changes end-to-end: bundled-tool routing can hide config drift. Use `vp check --fix` only when you intend to apply formatting or lint fixes. Documentation-only changes need no unrelated test suites.

### CLI Snapshots

Write new CLI tests, including interactive flows, in `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/`. Read the [snapshot runner instructions](crates/vp_cli_snapshots/tests/cli_snapshots/README.md) before adding cases; they define `snapshots.toml`, CLI flavors, `vpt` helpers, and prompt milestones. See [the RFC](rfcs/interactive-snapshot-tests.md) for design rationale.

- Steps are argv arrays without an implicit shell. Use `vpt` helpers instead of coreutils for portable file operations.
- Local-flavor cases require a fresh `packages/cli/dist`. Use `just snapshot-test-global <filter>` when no JS build is available.
- For intended snapshot changes, run `UPDATE_SNAPSHOTS=1 just snapshot-test <filter>`. Review and commit the recorded `.md` files with the fixture; mismatches fail the run and produce `.md.new` files.

## Pull Requests

Prefer a stack of small PRs for work with separable layers. Use the `gh-stack` extension or github.com; stacks require branches in this repository, so use standalone PRs from forks. Follow [CONTRIBUTING.md](CONTRIBUTING.md#submitting-pull-requests) for submission and commit-signing requirements.

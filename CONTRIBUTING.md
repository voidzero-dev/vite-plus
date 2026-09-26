# Contributing Guide

## Initial Setup

You'll need the following tools installed on your system:

- Node.js (version specified in [`.node-version`](.node-version))
- pnpm (version specified in the `packageManager` field of [`package.json`](package.json))
- Just
- CMake
- Rust and Cargo
- cargo-binstall

If you haven't installed Node.js and pnpm, we recommend using Vite+ to manage them. See [environment management](https://viteplus.dev/guide/env) for details.

### macOS / Linux

If you haven't installed Node.js and pnpm, we recommend installing them with Vite+:

```bash
curl -fsSL https://vite.plus | VP_NODE_MANAGER=yes VP_PM_MANAGER=yes bash
```

After installing Vite+, restart your terminal to activate the `node` and `pnpm` shims.

You'll also need Just and CMake:

```bash
brew install just cmake
```

Install Rust & Cargo using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install cargo-binstall
```

Initial setup to install dependencies for Vite+:

```bash
just init
```

### Windows

If you haven't installed Node.js and pnpm, we recommend installing them with Vite+:

```powershell
$env:VP_NODE_MANAGER = "yes"
$env:VP_PM_MANAGER = "yes"
irm https://viteplus.dev/install.ps1 | iex
```

After installing Vite+, restart your terminal to activate the `node` and `pnpm` shims.

You'll also need Just and CMake. You can install them using [winget](https://learn.microsoft.com/en-us/windows/package-manager/):

```powershell
winget install Casey.Just Kitware.CMake
```

Install Rust & Cargo from [rustup.rs](https://rustup.rs/), then install `cargo-binstall`:

```powershell
cargo install cargo-binstall
```

Initial setup to install dependencies for Vite+:

```powershell
just init
```

**Note:** Run commands in PowerShell or Windows Terminal. Some commands may require elevated permissions.

## Build Vite+ and upstream dependencies

To create a release build of Vite+ and all upstream dependencies, run:

```bash
just build
```

## Git Hooks (optional)

Pre-commit checks are opt-in. Run `vp config` after a build to install the hook dispatcher; each commit then runs `vp staged` on staged files. `vp staged` loads its config from `vite.config.ts` and needs the built JS packages, so build first. Opt out with `vp hooks disable`, or skip one run with `VP_GIT_HOOKS=0`. If husky was active in your clone before, run `git config --unset core.hooksPath` once first.

## Install the Vite+ Global CLI from source code

```bash
pnpm bootstrap-cli
vp --version
```

This builds all packages, compiles the Rust `vp` binary, and installs the CLI to `~/.vite-plus`.

To switch back to a release version, use `vp upgrade --force` (`current` points to `local-dev-*` but the binary version may still match the release, so `--force` is needed)

```bash
vp upgrade --force
```

## Validate the local build against a real project

Automated tests don't cover everything. Complex flows such as prompts, pickers, and scaffolding are also worth validating by running your work-in-progress CLI inside a real Vite+ project.

First, understand how `vp` picks which `vite-plus` to run: for JS-backed commands (such as `vp create`), the global `vp` binary resolves `vite-plus` from the project's `node_modules` first and only falls back to the global installation in `~/.vite-plus`. If your test project has `vite-plus` installed from npm, `pnpm bootstrap-cli` alone will not make it run your local code.

Build the local CLI package after each change:

```bash
pnpm -F vite-plus build      # TypeScript + native NAPI binding
pnpm -F vite-plus build-ts   # faster, when only TypeScript changed
```

### `pnpm link` the local package

Link your checkout into the test project. The global `vp` then delegates to the project-local CLI, and re-entrant `vp` sub-commands (for example `vp create` running `vp install` and `vp fmt` after scaffolding) resolve back to the same linked checkout:

```bash
cd /path/to/test-project
pnpm link /path/to/vite-plus/packages/cli

vp create   # now runs your local checkout
```

Verify the link with `ls -l node_modules/vite-plus` (it should be a symlink into your checkout). Notes:

- pnpm records the link as a `vite-plus: link:...` override (in `pnpm-workspace.yaml` for workspace projects, otherwise under `pnpm.overrides` in `package.json`), so it survives later installs. Don't commit that override in the test project.
- `pnpm link` may also add a `packageManager` field to the test project's `package.json`; revert it if unwanted.
- Undo with `pnpm unlink vite-plus`, or remove the override and run `pnpm install`.

### Test `vp migrate` / `vp create` through a local npm registry

`pnpm link` swaps the code inside an existing project, but `vp migrate` and `vp create` pin the exact CLI version and then _install_ it. Use the local npm registry to make your built checkout available to these commands.

```bash
pnpm build   # the served packages are built artifacts; rebuild after JS changes

# One-shot: wrap any command (from the project you want to migrate)
cd /path/to/test-project
node /path/to/vite-plus/packages/tools/src/local-npm-registry.ts --pack -- vp migrate --no-interactive
node /path/to/vite-plus/packages/tools/src/local-npm-registry.ts --pack -- vp create vite:application --no-interactive

# Or keep a server running for repeated commands (from the vite-plus checkout)
pnpm local-registry --pack --serve
# copy the printed `export ...` lines into the shell where you run vp
```

`pnpm local-registry:ps` lists any registry processes still running; `pnpm local-registry:kill` stops them all and removes their leftover temp caches. See the [tool documentation](packages/tools/README.md#local-npm-registry) for implementation details and test integration.

### Global CLI (Rust) changes

`pnpm link` only swaps the JS side; the `vp` binary on `PATH` (and the Rust-backed commands it handles directly, such as package-manager commands) is still the installed binary. For changes to the Rust global CLI (`crates/`), follow [the source installation steps](#install-the-vite-global-cli-from-source-code), and combine with `pnpm link` when the change spans both layers.

## Workflow for build and test

You can run this command to build, test and check if there are any snapshot changes:

```bash
pnpm bootstrap-cli && pnpm test && git status
```

## CLI Snapshot Tests (PTY runner)

CLI output and interactive flows (prompts, pickers, keystrokes, ctrl-c) are tested with the PTY snapshot suite in `crates/vp_cli_snapshots/`. Every step runs in a real pseudo-terminal; snapshots are Markdown files compared with real pass/fail semantics. **Write new CLI tests here**, one fixture directory per scenario with a `snapshots.toml` declaring the cases.

```bash
# Build vp and run the whole suite
just snapshot-test

# Filter by trial name substring
just snapshot-test create

# Record or accept snapshot changes, then review the .md diffs like code
UPDATE_SNAPSHOTS=1 just snapshot-test create
```

The full case/step/interaction reference (including the `vpt` helper tool and milestone conventions for interactive tests) lives in `crates/vp_cli_snapshots/tests/cli_snapshots/README.md`; the design rationale is in `rfcs/interactive-snapshot-tests.md`.

## Submitting Pull Requests

Keep pull requests small and focused. Split changes that can be reviewed and merged independently into separate PRs. If one change depends on another, submit the dependent PR after its prerequisite has merged.

## Pull upstream dependencies

> [!NOTE]
>
> Upstream dependencies only need to be updated when an ["upgrade upstream dependencies"](https://github.com/voidzero-dev/vite-plus/pulls?q=is%3Apr+feat%28deps%29%3A+upgrade+upstream+dependencies+merged) pull request is merged.

To sync the latest upstream dependencies such as Rolldown and Vite, run:

```bash
pnpm tool sync-remote
just build
```

## macOS Performance Tip

If you are using macOS, add your terminal app (Ghostty, iTerm2, Terminal, …) to the approved "Developer Tools" apps in the Privacy panel of System Settings and restart your terminal app. Your Rust builds will be about ~30% faster.

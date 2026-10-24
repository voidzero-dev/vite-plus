# Contributing Guide

## Initial Setup

You'll need the following tools installed on your system:

- Node.js (version specified in [`.node-version`](.node-version))
- pnpm (version specified in the `packageManager` field of [`package.json`](package.json))
- Just
- CMake
- Rust and Cargo
- cargo-binstall
- [mbx](#shared-rust-build-cache) for shared Rust build artifacts

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

Set up [mbx](#shared-rust-build-cache), then install dependencies for Vite+:

```bash
just init
```

### Nix (optional)

If you use [Nix](https://nixos.org/) with flakes enabled, you can use the dev shell defined in [`flake.nix`](flake.nix) instead of installing the tools above manually:

```bash
nix develop
just init
```

The dev shell includes mbx and routes Cargo through it. Skip the Cargo installation and `mbx setup` steps below.

Known limitation: on NixOS, the browser-mode cases of `just snapshot-test` do not run because the Chromium binary downloaded by `playwright install` cannot start without its system libraries. These cases are covered by CI; every other check works in the dev shell.

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

Set up [mbx](#shared-rust-build-cache), then install dependencies for Vite+:

```powershell
just init
```

**Note:** Run commands in PowerShell or Windows Terminal. Some commands may require elevated permissions.

### Shared Rust build cache

The contributor setup uses [mbx](https://mr-boxington.jdx.dev/) to reuse Rust build artifacts across checkouts and worktrees. Install it with Cargo and set up its Cargo shim once per machine:

```sh
cargo install mbx --locked
mbx setup
```

Apply the `PATH` change printed by `mbx setup`, and save it in your shell profile or user `PATH`. The shim directory must come before the Rust toolchain's Cargo directory. This makes ordinary Cargo commands use mbx, including calls from Just, pnpm, and native binding builds. The setup also configures rust-analyzer's user-level check command when no check settings exist. Restart terminals and editors after changing `PATH`.

Check that `command -v cargo` on macOS/Linux, or `(Get-Command cargo).Path` in PowerShell, points to the shim. Then check the installation:

```bash
mbx setup --status
mbx doctor
```

If you use `sccache`, remove its local Rust wrapper configuration before using mbx; see the [migration instructions](https://mr-boxington.jdx.dev/cookbook/migrate#from-sccache). The Nix shell clears inherited `RUSTC_WRAPPER` and `CARGO_BUILD_RUSTC_WRAPPER` values automatically. Keep the existing CI cache configuration unchanged.

Run the build commands below normally, including in new worktrees. Each worktree keeps a separate managed target directory; mbx shares cached artifacts between them. Do not point multiple worktrees at one `CARGO_TARGET_DIR`. The Rust toolchain stays pinned by [`rust-toolchain.toml`](rust-toolchain.toml).

Start with mbx's [disk-scaled defaults](https://mr-boxington.jdx.dev/managed-targets#budgets-scale-with-the-disk); this repository does not prescribe a fixed cache size. Use `mbx stats` to inspect reuse. If you need a fixed budget across your projects, configure [`gc.max_total_size`](https://mr-boxington.jdx.dev/configuration#single-cache-budget) for your available disk space and workload. This user-level setting uses logical size and is not a hard limit on physical disk use; `MBX_GC_MAX_TOTAL_SIZE` can override it for a command.

The first build normally moves an existing `target` directory under mbx management and preserves its contents; [`mbx adopt --dry-run`](https://mr-boxington.jdx.dev/cli/adopt) previews adoption.

To bypass mbx for troubleshooting, set `MBX_DISABLE=1` for the command. See the [upstream setup guide](https://mr-boxington.jdx.dev/setup) for editor configuration and removal.

#### Reclaiming disk space

Keep automatic collection enabled for normal development. Manual cleanup is useful when disk space is low, after removing many worktrees, or when you no longer need builds from old toolchains or profiles. Avoid cleaning before every build: cached artifacts reduce repeated compilation.

Inspect cache use and preview collection:

```sh
mbx cache stats
mbx cache projects
mbx gc --dry-run
```

Run collection when you want to reclaim eligible state across the cache:

```sh
mbx gc
```

[`mbx gc`](https://mr-boxington.jdx.dev/cli/gc) collects targets from deleted checkouts and evicts eligible build state and cached objects according to age, size, and free-space limits. It can remove nothing when those limits are satisfied. Under disk pressure, the actual removal can differ from the preview. Active builds retain their state.

To discard the managed build state for one unused worktree, including nested Rust workspaces such as `rolldown`, use its absolute path:

```sh
mbx clean --under /absolute/path/to/unused-worktree
```

[`mbx clean`](https://mr-boxington.jdx.dev/cli/clean) removes managed targets and private incremental state, but keeps shared cached artifacts. The path can already be deleted. The next build restores matching cached outputs and recompiles anything unavailable. `cargo clean` does not remove mbx's private incremental state, and deleting a `target` symlink alone does not reclaim its stored artifacts.

Cache reports use logical sizes. Shared filesystem blocks can make the physical space reclaimed smaller than the reported sizes; check disk free space to see the actual change.

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

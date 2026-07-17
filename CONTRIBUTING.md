# Contributing Guide

## Initial Setup

### macOS / Linux

You'll need the following tools installed on your system:

```bash
brew install pnpm node just cmake
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

You'll need the following tools installed on your system. You can use [winget](https://learn.microsoft.com/en-us/windows/package-manager/).

```powershell
winget install pnpm.pnpm OpenJS.NodeJS.LTS Casey.Just Kitware.CMake
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

`pnpm link` swaps the code inside an existing project, but `vp migrate` and `vp create` pin the exact CLI version and then _install_ it, so the checkout's `vite-plus` / `@voidzero-dev/vite-plus-core` must be resolvable from a registry. `packages/tools/src/local-npm-registry.ts` provides that: it packs the checkout, serves the tarballs behind a real registry HTTP interface, and proxies every other package upstream. This replaces the old pkg.pr.new publish + registry-bridge round-trip for local iteration; you can verify migrate/create logic immediately after a build.

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

Notes:

- The served versions carry an old publish time, so `minimumReleaseAge` gates never quarantine them, and wrapped runs get throwaway Yarn Berry / bun caches (both cache registry state in ways that would otherwise leak stale local builds between runs).
- The same server backs PTY snapshot cases with `local-registry = true` and ecosystem e2e (`ecosystem-ci/patch-project.ts`), so a flow that works here works there too.
- `pnpm local-registry:ps` lists any registry processes still running (e.g. a `--serve` you forgot, or a wrapper that was killed mid-run); `pnpm local-registry:kill` stops them all and removes their leftover temp caches.

### Global CLI (Rust) changes

`pnpm link` only swaps the JS side; the `vp` binary on `PATH` (and the Rust-backed commands it handles directly, such as package-manager commands) is still whatever is installed in `~/.vite-plus`. For changes to the Rust global CLI (`crates/`), install it from source, and combine with `pnpm link` when the change spans both layers:

```bash
pnpm bootstrap-cli
vp --version
```

## Workflow for build and test

You can run this command to build, test and check if there are any snapshot changes:

```bash
pnpm bootstrap-cli && pnpm test && git status
```

## CLI Snapshot Tests (PTY runner)

CLI output and interactive flows (prompts, pickers, keystrokes, ctrl-c) are tested with the PTY snapshot suite in `crates/vite_cli_snapshots/`. Every step runs in a real pseudo-terminal; snapshots are Markdown files compared with real pass/fail semantics. **Write new CLI tests here**, one fixture directory per scenario with a `snapshots.toml` declaring the cases.

```bash
# Build vp and run the whole suite
just snapshot-test

# Filter by trial name substring
just snapshot-test create

# Record or accept snapshot changes, then review the .md diffs like code
UPDATE_SNAPSHOTS=1 just snapshot-test create
```

The full case/step/interaction reference (including the `vpt` helper tool and milestone conventions for interactive tests) lives in `crates/vite_cli_snapshots/tests/cli_snapshots/README.md`; the design rationale is in `rfcs/interactive-snapshot-tests.md`.

## Verified Commits

All commits in PR branches should be GitHub-verified so reviewers can confirm commit authenticity.

Set up local commit signing and GitHub verification first:

- Follow GitHub's guide for GPG commit signature verification: https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification#gpg-commit-signature-verification
- If you use Graphite, add the Graphite GPG key to your GitHub account from the Graphite UI as well, otherwise commits updated by Graphite won't show as verified.

After setup, re-sign any existing commits in your branch so the full branch is verified:

```bash
# Re-sign each commit on your branch (replace origin/main with your branch base if needed)
git rebase -i origin/main
# At each stop:
git commit --amend --date=now --no-edit -S
# Then continue:
git rebase --continue
```

When done, force-push the updated branch history:

```bash
git push --force-with-lease
```

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

# Contributing Guide

## Initial Setup

### macOS / Linux

You'll need the following tools installed on your system:

```
brew install pnpm node just cmake
```

Install Rust & Cargo using rustup:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install cargo-binstall
```

Initial setup to install dependencies for Vite+:

```
just init
```

`just init` bootstraps the pinned `rolldown` and `vite` source trees needed for local development, then installs workspace dependencies.

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

`just init` bootstraps the pinned `rolldown` and `vite` source trees needed for local development, then installs workspace dependencies.

**Note:** Run commands in PowerShell or Windows Terminal. Some commands may require elevated permissions.

## Build Vite+ and upstream dependencies

To create a release build of Vite+ and all upstream dependencies, run:

```
just build
```

## Install the Vite+ Global CLI from source code

```
pnpm bootstrap-cli
vp --version
```

This builds all packages, compiles the Rust `vp` binary, and installs the CLI to `~/.vite-plus`.

## Workflow for build and test

You can run this command to build, test and check if there are any snapshot changes:

```
pnpm bootstrap-cli && pnpm test && git status
```

## Running Snap Tests

Snap tests verify CLI output. They are located in `packages/cli/snap-tests/` (local CLI) and `packages/cli/snap-tests-global/` (global CLI).

```bash
# Run all snap tests (local + global)
pnpm -F vite-plus snap-test

# Run only local CLI snap tests
pnpm -F vite-plus snap-test-local
pnpm -F vite-plus snap-test-local <name-filter>

# Run only global CLI snap tests
pnpm -F vite-plus snap-test-global
pnpm -F vite-plus snap-test-global <name-filter>
```

Snap tests auto-generate `snap.txt` files. Check `git diff` to verify output changes are correct.

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

```
pnpm tool sync-remote
just build
```

## macOS Performance Tip

If you are using macOS, add your terminal app (Ghostty, iTerm2, Terminal, …) to the approved "Developer Tools" apps in the Privacy panel of System Settings and restart your terminal app. Your Rust builds will be about ~30% faster.

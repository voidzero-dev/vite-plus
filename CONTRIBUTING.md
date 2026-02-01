# Contributing Guide

## Initial Setup

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

## Build Vite+ and upstream dependencies

To create a release build of Vite+ and all upstream dependencies, run:

```
just build
```

## Install the Vite+ Global CLI from source code

```
pnpm bootstrap-cli
vp-dev --version
```

Note: Local development installs the CLI as `vp-dev` (package name: `vite-plus-cli-dev`) to avoid overriding the published `vite-plus-cli` package and its `vp` bin name. In CI, `pnpm bootstrap-cli:ci` installs it as `vp`.

## Workflow for build and test

You can run this command to build, test and check if there are any snapshot changes:

```
pnpm bootstrap-cli && pnpm test && git status
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

## Testing install.sh locally

To test the install script with a locally built binary instead of downloading from npm:

```bash
# Build the vp binary
pnpm bootstrap-cli

# Run install.sh with the local binary
VITE_PLUS_LOCAL_BINARY=./target/release/vp bash ./packages/global/install.sh

# Verify the installation
~/.vite-plus/current/bin/vp --version
```

For fully offline testing (skip all npm downloads):

```bash
# Build the vp binary and JS bundle
pnpm bootstrap-cli

# Run install.sh with local binary and package
VITE_PLUS_LOCAL_BINARY=./target/release/vp \
VITE_PLUS_LOCAL_PACKAGE=./packages/global \
bash ./packages/global/install.sh
```

This is useful when making changes to `install.sh` and want to verify it works correctly before publishing.

## macOS Performance Tip

If you are using macOS, add your terminal app (Ghostty, iTerm2, Terminal, …) to the approved "Developer Tools" apps in the Privacy panel of System Settings and restart your terminal app. Your Rust builds will be about ~30% faster.

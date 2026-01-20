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

## Install Vite+ Global CLI from source code

```
pnpm bootstrap-cli
vp --version
```

Note: Local development installs the CLI as `vp` (package name: `vite-plus-cli-dev`) to avoid overriding the published `vite-plus-cli` package and its `vite` bin name. In CI, `pnpm bootstrap-cli:ci` installs it as `vite`.

## Workflow for build and test

One command to run build, unit tests, snap tests and check if there are any changes:

```
pnpm bootstrap-cli && pnpm test && git status
```

## Pull upstream dependencies (On-demand)

> It is only necessary to re-sync the upstream code after the ["upgrade upstream dependencies"](https://github.com/voidzero-dev/vite-plus/pulls?q=is%3Apr+feat%28deps%29%3A+upgrade+upstream+dependencies+merged) pull request has been merged.

When you want to pull the latest upstream dependencies such as Rolldown and Vite, run:

```
pnpm tool sync-remote

# build all packages again
just build
```

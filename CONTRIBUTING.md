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

## Pull upstream dependencies

When you want to pull the latest upstream dependencies such as Rolldown and Vite, run:

```
node packages/tools/src/index.ts sync-remote
pnpm install
```

## Install Vite+ Global CLI from source code

```
pnpm bootstrap-cli
vite --version
```

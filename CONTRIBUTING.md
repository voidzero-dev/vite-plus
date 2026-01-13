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

## Install internal global cli

Add the following lines to your `~/.npmrc` file:

```
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
@voidzero-dev:registry=https://npm.pkg.github.com/
```

Create a classic personal access token, following this guide: https://docs.github.com/en/packages/learn-github-packages/about-permissions-for-github-packages#about-scopes-and-permissions-for-package-registries

Use this token to install the global cli:

```
GITHUB_TOKEN=<your-token> npm install -g @voidzero-dev/global
```

Use 1Password cli:

```
GITHUB_TOKEN=$(op read "op://YOUR_GITHUB_TOKEN_PATH") npm install -g @voidzero-dev/global
```

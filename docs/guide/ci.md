# Continuous Integration

You can use `voidzero-dev/setup-vp` to use Vite+ in CI environments.

## Overview

For GitHub Actions, the recommended setup is [`voidzero-dev/setup-vp`](https://github.com/voidzero-dev/setup-vp). It installs Vite+, sets up the required Node.js version and package manager, and can cache package installs automatically.

That means you usually do not need separate `setup-node`, package-manager setup, and manual dependency-cache steps in your workflow.

## GitHub Actions

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@v1
  with:
    node-version: '24'
    cache: true
- run: vp install
- run: vp check
- run: vp test
- run: vp build
```

With `cache: true`, `setup-vp` handles dependency caching for you automatically.

## Simplifying Existing Workflows

If you are migrating an existing GitHub Actions workflow, you can often replace large blocks of Node, package-manager, and cache setup with a single `setup-vp` step.

#### Before:

```yaml [.github/workflows/ci.yml]
- uses: pnpm/action-setup@v6
  with:
    version: 11

- uses: actions/setup-node@v6
  with:
    node-version: '24'
    cache: pnpm

- run: pnpm ci && pnpm dev:setup
- run: pnpm check
- run: pnpm test
```

#### After:

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@v1
  with:
    node-version: '24'
    cache: true

- run: vp install && vp run dev:setup
- run: vp check
- run: vp test
```

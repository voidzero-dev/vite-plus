# Continuous Integration

You can use `voidzero-dev/setup-vp` to use Vite+ in CI environments.

## Overview

For GitHub Actions, the recommended setup is [`voidzero-dev/setup-vp`](https://github.com/voidzero-dev/setup-vp). It installs Vite+, sets up the required Node.js version and package manager, and can cache package installs automatically.

That means you usually do not need separate `setup-node`, package-manager setup, and manual dependency-cache steps in your workflow.

## GitHub Actions

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@v1
  with:
    node-version: "24"
    cache: true
- run: vp install
- run: vp check
- run: vp test
- run: vp build
```

With `cache: true`, `setup-vp` handles dependency caching for you automatically.

## npm v12 Readiness

npm v12 changes dependency install defaults so lifecycle scripts, git dependencies, and remote URL dependencies require explicit approval. If your Vite+ project uses npm in CI, you can test the stricter behavior early with npm 11.16 or newer:

```yaml [.github/workflows/ci.yml]
- run: npm install --strict-allow-scripts
- run: npm approve-scripts --allow-scripts-pending
```

Review any pending packages, commit the resulting `package.json` allowlist from `npm approve-scripts`, and add the required `--allow-git` or `--allow-remote` policy if your project depends on non-registry sources. See GitHub's [npm v12 breaking changes](https://github.blog/changelog/2026-06-09-upcoming-breaking-changes-for-npm-v12/) for the current migration guidance.

## Simplifying Existing Workflows

If you are migrating an existing GitHub Actions workflow, you can often replace large blocks of Node, package-manager, and cache setup with a single `setup-vp` step.

#### Before:

```yaml [.github/workflows/ci.yml]
- uses: pnpm/action-setup@v6
  with:
    version: 11

- uses: actions/setup-node@v6
  with:
    node-version: "24"
    cache: pnpm

- run: pnpm ci && pnpm dev:setup
- run: pnpm check
- run: pnpm test
```

#### After:

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@v1
  with:
    node-version: "24"
    cache: true

- run: vp install && vp run dev:setup
- run: vp check
- run: vp test
```

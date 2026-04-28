# Installing Dependencies

`vp install` installs dependencies using the current workspace's package manager.

## Overview

Use Vite+ to manage dependencies across pnpm, npm, Yarn, and Bun. Instead of switching between `pnpm install`, `npm install`, `yarn install`, and `bun install`, you can keep using `vp install`, `vp add`, `vp remove`, and the rest of the Vite+ package-management commands.

Vite+ detects the package manager from the workspace root in this order:

1. `packageManager` in `package.json`
2. `pnpm-workspace.yaml`
3. `pnpm-lock.yaml`
4. `yarn.lock` or `.yarnrc.yml`
5. `package-lock.json`
6. `bun.lock` or `bun.lockb`
7. `.pnpmfile.cjs` or `pnpmfile.cjs`
8. `bunfig.toml`
9. `yarn.config.cjs`

If none of those files are present, `vp` falls back to `pnpm` by default. Vite+ automatically downloads the matching package manager and uses it for the command you ran.

## Usage

```bash
vp install
```

Common install flows:

```bash
vp install
vp install --frozen-lockfile
vp install --lockfile-only
vp install --filter web
vp install -w
```

`vp install` maps to the correct underlying install behavior for the detected package manager, including the right lockfile flags for pnpm, npm, Yarn, and Bun.

## Global Packages

Use the `-g` flag for installing, updating or removing globally installed packages:

- `vp install -g <pkg>` installs a package globally
- `vp uninstall -g <pkg>` removes a global package
- `vp update -g [pkg]` updates one global package or all of them
- `vp list -g [pkg]` lists global packages

## Managing Dependencies

Vite+ provides all the familiar package management commands:

- `vp install` installs the current dependency graph for the project
- `vp add <pkg>` adds packages to `dependencies`, use `-D` for `devDependencies`
- `vp remove <pkg>` removes packages
- `vp update` updates dependencies
- `vp dedupe` reduces duplicate dependency entries where the package manager supports it
- `vp outdated` shows available updates
- `vp list` shows installed packages
- `vp why <pkg>` explains why a package is present
- `vp info <pkg>` shows registry metadata for a package
- `vp rebuild` rebuilds native modules (e.g. after switching Node.js versions)
- `vp link` and `vp unlink` manage local package links
- `vp dlx <pkg>` runs a package binary without adding it to the project
- `vp pm <command>` forwards a raw package-manager-specific command when you need behavior outside the normalized `vp` command set

### Command Guide

#### Install

Use `vp install` when you want to install exactly what the current `package.json` and lockfile describe.

- `vp install` is the standard install command
- `vp install --frozen-lockfile` fails if the lockfile would need changes
- `vp install --no-frozen-lockfile` allows lockfile updates explicitly
- `vp install --lockfile-only` updates the lockfile without performing a full install
- `vp install --prefer-offline` and `vp install --offline` prefer or require cached packages
- `vp install --ignore-scripts` skips lifecycle scripts
- `vp install --filter <pattern>` scopes install work in monorepos
- `vp install -w` installs in the workspace root

#### Global Install

Use these commands when you want package-manager-managed tools available outside a single project.

- `vp install -g typescript`
- `vp uninstall -g typescript`
- `vp update -g`
- `vp list -g`

#### Add and Remove

Use `vp add` and `vp remove` for day-to-day dependency edits instead of editing `package.json` by hand.

- `vp add react`
- `vp add -D typescript vitest`
- `vp add -O fsevents`
- `vp add --save-peer react`
- `vp remove react`
- `vp remove --filter web react`

#### Update, Dedupe, and Outdated

Use these commands to maintain the dependency graph over time.

- `vp update` refreshes packages to newer versions
- `vp outdated` shows which packages have newer versions available
- `vp dedupe` asks the package manager to collapse duplicates where possible

#### Inspect

Use these when you need to understand the current state of dependencies.

- `vp list` shows installed packages
- `vp why react` explains why `react` is installed
- `vp info react` shows registry metadata such as versions and dist-tags

#### Rebuild

Use `vp rebuild` when native modules need to be recompiled, for example after switching Node.js versions or when a C/C++ addon fails to load.

- `vp rebuild` rebuilds all native modules
- `vp rebuild -- <args>` passes extra arguments to the underlying package manager

```bash
vp rebuild
vp rebuild -- --update-binary
```

`vp rebuild` is a shorthand for `vp pm rebuild`.

#### Advanced

Use these when you need lower-level package-manager behavior.

- `vp link` and `vp unlink` manage local development links
- `vp dlx create-vite` runs a package binary without saving it as a dependency
- `vp pm <command>` forwards directly to the resolved package manager

Examples:

```bash
vp pm config get registry
vp pm cache clean --force
vp pm exec tsc --version
```

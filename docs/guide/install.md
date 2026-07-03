# Installing Dependencies

`vp install` installs dependencies using the current workspace's package manager.

## Overview

Use Vite+ to manage dependencies across pnpm, npm, Yarn, and Bun. Instead of switching between `pnpm install`, `npm install`, `yarn install`, and `bun install`, you can keep using `vp install`, `vp add`, `vp remove`, and the rest of the Vite+ package-management commands.

Vite+ detects the package manager from the workspace root in this order:

1. `packageManager` in `package.json`
2. `devEngines.packageManager` in `package.json`
3. `pnpm-workspace.yaml`
4. `pnpm-lock.yaml`
5. `yarn.lock` or `.yarnrc.yml`
6. `package-lock.json`
7. `bun.lock` or `bun.lockb`
8. `.pnpmfile.cjs` or `pnpmfile.cjs`
9. `bunfig.toml`
10. `yarn.config.cjs`

If none of those files are present, `vp` falls back to `pnpm` by default. Vite+ automatically downloads the matching package manager and uses it for the command you ran. When detection comes from lockfiles or config files, the resolved version is written to `devEngines.packageManager` so future runs are deterministic; projects that already declare `packageManager` or `devEngines.packageManager` are left as-is.

The [`devEngines.packageManager`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devengines) field accepts a single object or an array of objects, and its `version` may be a semver range:

```json
{
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "^11.0.0",
      "onFail": "download"
    }
  }
}
```

A range resolves to an already-downloaded satisfying version when possible, otherwise to the latest satisfying version from the npm registry. The range itself stays the source of truth; Vite+ never freezes it into an exact `packageManager` pin. When both `packageManager` and `devEngines.packageManager` are declared, the `packageManager` field drives selection and Vite+ warns when it does not satisfy the devEngines constraint (`vp env doctor` shows details).

Vite+ currently downloads the declared package manager (the `onFail: "download"` behavior); the other `onFail` values are accepted but not yet differentiated.

The explicit `packageManager` field (or the `devEngines.packageManager` declaration) also affects matching package-manager shims. If a project has `packageManager: "npm@10.9.4"`, `npm` and `npx` use npm 10.9.4. Other generated alias pairs behave the same way: `pnpm`/`pnpx`, `yarn`/`yarnpkg`, and `bun`/`bunx`. Mismatched tools are not translated; `npm` in a `pnpm` project still resolves as npm.

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
- `vp outdated -g [pkg]` prints outdated packages

::: warning
These commands do **NOT** interact with the underlying package manager's global installation directory.

Instead, Vite+ manages its own global packages under `VP_HOME/packages`, allowing them to remain available across different Node.js versions.

As a result, commands such as `vp link` do not affect Vite+'s global packages and will not appear in `vp list -g`.
:::

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
- `vp outdated -g`

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
- `vp rebuild <package...>` rebuilds the listed packages only
- `vp rebuild -- <args>` passes extra arguments to the underlying package manager

```bash
vp rebuild
vp rebuild better-sqlite3 sharp
vp rebuild -- --update-binary
```

`vp rebuild` is a shorthand for `vp pm rebuild`.

With pnpm v10+, bare `vp rebuild` only rebuilds packages whose build scripts are listed in `onlyBuiltDependencies` (or approved via `pnpm approve-builds`); name the package explicitly to force a rebuild that bypasses the approval gate.

#### Advanced

Use these when you need lower-level package-manager behavior.

- `vp link` and `vp unlink` manage local development links
- `vp dlx create-vite` runs a package binary without saving it as a dependency
- `vp pm <command>` forwards directly to the resolved package manager

Examples:

```bash
vp pm config get registry
vp pm cache clean -- --force
vp pm audit --json
```

#### Staged publishing

`vp pm stage` exposes [npm's staged publishing](https://docs.npmjs.com/staged-publishing) workflow: a build is uploaded to a staging area (no 2FA, CI-friendly), then a maintainer approves or rejects it from a trusted device (2FA). It adapts to the detected package manager.

```bash
vp pm stage publish              # upload the package to staging (no 2FA)
vp pm stage list                 # list staged versions
vp pm stage view <stage-id>      # inspect a staged version
vp pm stage download <stage-id>  # download the staged tarball
vp pm stage approve <stage-id>   # promote to the live registry (2FA)
vp pm stage reject <stage-id>    # discard a staged version (2FA)
```

- pnpm (`pnpm stage`, requires pnpm ≥ 11.3) and npm (`npm stage`, requires npm ≥ 11.15 and Node ≥ 22.14) pass through directly.
- yarn (Berry) uses its npm plugin (`yarn npm publish --staged`, `yarn npm stage …`); `view`/`download` fall back to npm.
- yarn Classic and bun have no staged-publishing support and fall back to `npm stage`.

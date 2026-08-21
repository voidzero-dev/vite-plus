# Upgrading Vite+

Use `vp upgrade` to update the global `vp` binary, and use Vite+'s package management commands to update the local `vite-plus` package in a project.

## Overview

There are two parts to upgrading Vite+:

- The global `vp` command installed on your machine
- The local `vite-plus` package used by an individual project

You can upgrade both of them independently.

## Show the Toolchain

Run `vp toolchain` to show the components for the current directory:

```bash
vp toolchain
vp toolchain vite
vp toolchain vite rolldown oxc
vp toolchain --json
```

The command uses the local `vite-plus` package when the project has one. Use
`--global` to show the release for the global `vp` command:

```bash
vp toolchain --global
```

`vp why <package>` shows the dependency graph from the package manager. It
cannot show code bundled into `@voidzero-dev/vite-plus-core`. It also cannot
show engines compiled into Vite+. Use `vp toolchain` to show those versions and
relationships.

## Global `vp`

```bash
vp upgrade                        # upgrade to the latest version
vp upgrade --check                # check for updates without installing
vp upgrade <version>              # install a specific version
vp upgrade --registry <registry>  # use a custom npm registry
```

### Move an Existing Install to the Split Directory Layout

Vite+ 0.3.0 is the first release that supports the split directory layout.
Vite+ 0.2.x and earlier use the single-root layout for fresh installs and
upgrades.

`vp upgrade` keeps an existing default install in `~/.vite-plus` on Unix or
`%USERPROFILE%\.vite-plus` on Windows. The command upgrades the CLI in that
directory. It does not move the install to the split platform directories. You
can continue to use the existing layout.

To use the split layout now, remove the existing install. Then install Vite+
again. Run `vp implode` in a shell that uses the current install. The command
removes the generated environment file and shell profile entries. It does not
unset directory variables in the current shell. Unset all Vite+ directory
variables before you run the installer. This can include values from an earlier
preview environment file. Alternatively, start a new shell after `vp implode`.
Then run the installer in the new shell.

::: warning
`vp implode` removes all Vite+-managed Node.js runtimes, global packages,
configuration, and caches. Keep the existing layout if you do not want to
recreate that data.
:::

```bash
vp implode
unset VP_HOME VP_DATA_DIR VP_BIN_DIR VP_CACHE_DIR
curl -fsSL https://vite.plus | bash
```

On Windows:

```powershell
vp implode
Remove-Item Env:\VP_HOME, Env:\VP_DATA_DIR, Env:\VP_BIN_DIR, Env:\VP_CACHE_DIR -ErrorAction SilentlyContinue
irm https://vite.plus/ps1 | iex
```

Also remove persistent definitions of `VP_HOME`, `VP_DATA_DIR`, `VP_BIN_DIR`,
and `VP_CACHE_DIR` from your shell profile or system environment. A fresh
install uses `VP_HOME` or a complete `VP_*_DIR` group that remains set.
`VP_HOME` selects the single-root layout. If you install Vite+ 0.2.x or earlier,
the installer also uses this layout. The installer prints a notice.

### Rollback

Vite+ keeps the **3 most recent** versions installed so you can revert quickly:

```bash
vp upgrade --rollback
```

Older versions are pruned automatically after each upgrade. The active version and the previous version are always kept, so a rollback target is never removed.

## Local `vite-plus`

The recommended way to upgrade an existing Vite+ project is `vp migrate`:

```bash
vp migrate
```

On a project that is already on Vite+, migrate does a toolchain version upgrade only: it re-pins `vite-plus`, the `vite` -> `@voidzero-dev/vite-plus-core` alias, and the `vitest` pin to the versions the global `vp` now bundles, across every workspace package. It skips the first-time setup steps (git hooks, editor and agent files, lint migration), so a version bump does not re-touch things you already configured. Pass `--full` to also run that setup.

### Updating the Vitest Pin

If you migrated with `vp migrate`, your project pins `vitest` to an exact version so the whole project shares a single Vitest copy with the bundled `vp test` runner. The pin lives in your package manager's override block:

- **npm / Bun:** a `vitest` entry under `overrides` in `package.json`
- **Yarn:** a `vitest` entry under `resolutions` in `package.json`
- **pnpm:** a `vitest@*` entry under `overrides` in `pnpm-workspace.yaml`. If your `package.json` already has a `pnpm` field, the entry lives under `pnpm.overrides` in `package.json` instead. pnpm ignores `pnpm-workspace.yaml` overrides when `package.json` defines `pnpm.overrides`.

Under pnpm the managed keys use an explicit `@*` range (`vite@*`, `vitest@*`). pnpm applies an override by replacing the declared spec on every manifest, importer manifests included. A bare key matches any spec, including `catalog:`. The `@*` range keeps the override on the semver ranges that transitive and peer declarations use, and leaves `catalog:` references intact. `vp up` therefore no longer rewrites them to a concrete version.

A Vite+ release can bump the bundled Vitest. Because that pin also applies to `vite-plus`'s own `vitest` dependency, an out-of-date pin keeps installing the previous runner even after you upgrade `vite-plus` — splitting Vitest's internals (mocks, `expect`, runner state) between the pinned copy and the one `vp test` loads.

After upgrading `vite-plus`, re-pin `vitest` to the version Vite+ now bundles. Check that version with:

```bash
vp toolchain vitest
```

Then set the `vitest` override to that exact version, or rerun `vp migrate` to update the pin for you.

## Preview Builds

Some Vite+ pull requests publish temporary packages for testing before an npm release. Treat these as nightly or bleeding-edge builds: they are useful when you want to verify a specific fix, test a fresh upstream dependency bump, or confirm a change before the next release. For day-to-day work, prefer the published `latest` release.

Each commit on an eligible pull request is published to the [registry bridge](https://registry-bridge.viteplus.dev/). The bridge serves these builds as ordinary npm versions of the form `0.0.0-commit.<sha>` and proxies every other package to the npm registry. That means you install a preview with normal version specs instead of mutable URLs, and the same versions resolve in CI.

Both `vite-plus` and `@voidzero-dev/vite-plus-core` publish under the same `0.0.0-commit.<sha>` version. Each pull request carries a comment listing the exact version for its latest commit, along with ready-to-copy install steps.

You can find preview builds in pull requests that automatically update upstream dependencies. For examples, search the merged pull requests for [upstream dependency updates](https://github.com/voidzero-dev/vite-plus/pulls?q=is%3Apr+is%3Amerged+upgrade+upstream+dependencies).

Preview builds are addressed by pull request number or commit SHA. They are not a stable version range, and you should avoid leaving them in long-lived branches unless a maintainer asks you to.

### Global `vp` Preview

Install a preview build of the global CLI by passing `VP_PR_VERSION` to the installer. Pass a pull request number or a commit SHA:

```bash
curl -fsSL https://vite.plus | VP_PR_VERSION=<pr-or-sha> bash
```

On Windows:

```powershell
$env:VP_PR_VERSION = "<pr-or-sha>"
irm https://vite.plus/ps1 | iex
Remove-Item Env:\VP_PR_VERSION
```

The installer uses the registry bridge to resolve the ref to a `0.0.0-commit.<sha>` build. It installs this build like other versions. Run `vp toolchain --global` to show the active build and tool versions. After testing, run `vp upgrade --force` to restore the published release. You can also run the installer without `VP_PR_VERSION`.

### Local `vite-plus` Preview

After installing the preview global CLI above, run migrate in the project to move its local `vite-plus` onto the same build:

```bash
vp migrate
```

Migrate writes the bridge registry to `.npmrc`. For Yarn Berry, it writes the registry to `.yarnrc.yml`. It pins `vite-plus` and the `vite` alias to the matching `0.0.0-commit.<sha>` version. The `vite` alias points to `@voidzero-dev/vite-plus-core`. Commit the registry line if the project CI must test the preview.

After the install, run `vp toolchain` to show the selected versions. After testing, set `vite-plus` to `latest`. Remove the bridge `registry` line from `.npmrc` or `.yarnrc.yml`. Then run `vp install`.

# Upgrading Vite+

Use `vp upgrade` to update the global `vp` binary, and use Vite+'s package management commands to update the local `vite-plus` package in a project.

## Overview

There are two parts to upgrading Vite+:

- The global `vp` command installed on your machine
- The local `vite-plus` package used by an individual project

You can upgrade both of them independently.

## Global `vp`

```bash
vp upgrade                        # upgrade to the latest version
vp upgrade --check                # check for updates without installing
vp upgrade <version>              # install a specific version
vp upgrade --registry <registry>  # use a custom npm registry
```

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
- **pnpm:** a `vitest` entry under `overrides` in `pnpm-workspace.yaml` — unless your `package.json` already had a `pnpm` field, in which case it lives under `pnpm.overrides` in `package.json` instead (pnpm ignores `pnpm-workspace.yaml` overrides when `package.json` defines `pnpm.overrides`)

A Vite+ release can bump the bundled Vitest. Because that pin also applies to `vite-plus`'s own `vitest` dependency, an out-of-date pin keeps installing the previous runner even after you upgrade `vite-plus` — splitting Vitest's internals (mocks, `expect`, runner state) between the pinned copy and the one `vp test` loads.

After upgrading `vite-plus`, re-pin `vitest` to the version Vite+ now bundles. Check that version with:

```bash
vp --version
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

The installer resolves the ref to its `0.0.0-commit.<sha>` build through the registry bridge and installs it like any other version. Run `vp --version` afterward to confirm which build and bundled tool versions are active. When you are done testing, return to the published release with `vp upgrade --force` or by running the installer again without `VP_PR_VERSION`.

### Local `vite-plus` Preview

After installing the preview global CLI above, run migrate in the project to move its local `vite-plus` onto the same build:

```bash
vp migrate
```

Migrate points the project at the bridge registry (writing it to `.npmrc`, or `.yarnrc.yml` for Yarn Berry) and pins `vite-plus` and the `vite` -> `@voidzero-dev/vite-plus-core` alias to the matching `0.0.0-commit.<sha>` version. That registry line is what lets the same versions resolve in the project's own CI, so commit it if you want CI to test the preview too.

After installing, check the bundled versions with `vp --version`. When testing is complete, restore the published release: set `vite-plus` back to `latest`, remove the bridge `registry` line from `.npmrc` (or `.yarnrc.yml`), and reinstall with `vp install`.

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

Update the project dependency with the package manager commands in Vite+:

```bash
vp update vite-plus
```

You can also use `vp add vite-plus@latest` if you want to move the dependency explicitly to the latest version.

### Updating Aliased Packages

Vite+ sets up an npm alias for its core package during installation:

- `vite` is aliased to `npm:@voidzero-dev/vite-plus-core@latest`

`vp update vite-plus` does not re-resolve this alias in the lockfile. To fully upgrade, update it separately:

```bash
vp update @voidzero-dev/vite-plus-core
```

Or update everything at once:

```bash
vp update vite-plus @voidzero-dev/vite-plus-core
```

You can verify with `vp outdated` that no Vite+ packages remain outdated.

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

Some Vite+ pull requests publish temporary packages through `pkg.pr.new`. Treat these as nightly or bleeding-edge builds: they are useful when you want to verify a specific fix, test a fresh upstream dependency bump, or confirm a change before the next npm release. For day-to-day work, prefer the published `latest` release.

You can find preview builds in pull requests that automatically update upstream dependencies. For examples, search the merged pull requests for [upstream dependency updates](https://github.com/voidzero-dev/vite-plus/pulls?q=is%3Apr+is%3Amerged+upgrade+upstream+dependencies).

Preview builds are addressed by pull request number or commit SHA. They are not a stable version range, and you should avoid leaving them in long-lived branches unless a maintainer asks you to.

### Global `vp` Preview

Install a preview build of the global CLI by passing `VP_PR_VERSION` to the installer:

```bash
curl -fsSL https://vite.plus | VP_PR_VERSION=<pr-or-sha> bash
```

On Windows:

```powershell
$env:VP_PR_VERSION = "<pr-or-sha>"
irm https://vite.plus/ps1 | iex
Remove-Item Env:\VP_PR_VERSION
```

Run `vp --version` afterward to confirm which build and bundled tool versions are active. When you are done testing, return to the published release with `vp upgrade --force` or by running the installer again without `VP_PR_VERSION`.

### Local `vite-plus` Preview

To test an unreleased local package in a project, update the project dependency and any Vite+ alias or override before installing. Use the same pull request number or commit SHA for every preview URL:

- `vite-plus` should point at `https://pkg.pr.new/voidzero-dev/vite-plus@<pr-or-sha>`
- any direct `vite` alias or direct `@voidzero-dev/vite-plus-core` dependency should point at `https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@<pr-or-sha>`
- any `vite` override or resolution should point at that same core preview URL

For npm and Bun projects, update the relevant `package.json` entries:

```json
{
  "devDependencies": {
    "vite-plus": "https://pkg.pr.new/voidzero-dev/vite-plus@<pr-or-sha>",
    "vite": "https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@<pr-or-sha>"
  },
  "overrides": {
    "vite": "https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@<pr-or-sha>"
  }
}
```

Only include the direct `vite` entry if your project already has one. If your project has `@voidzero-dev/vite-plus-core` directly instead, update that package spec to the same core preview URL.

For pnpm workspaces, make the same temporary override change in `pnpm-workspace.yaml` if that is where your Vite+ overrides live:

```yaml
overrides:
  vite: 'https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@<pr-or-sha>'
```

For Yarn projects, update `resolutions` instead:

```json
{
  "resolutions": {
    "vite": "https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@<pr-or-sha>"
  }
}
```

Then install once:

```bash
vp install
```

After installing a preview, check the bundled versions with `vp --version`. If the preview includes a newer bundled Vitest, update your `vitest` override to that exact version so `vp test` and project imports keep using the same Vitest copy.

When testing is complete, restore every preview spec first: set `vite-plus` back to `latest`, and set any direct `vite` / `@voidzero-dev/vite-plus-core` dependency plus any `vite` override or resolution back to `npm:@voidzero-dev/vite-plus-core@latest`. Then reinstall:

```bash
vp install
```

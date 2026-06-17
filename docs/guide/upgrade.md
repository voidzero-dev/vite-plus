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

# Upgrading Vite+

Use `vp upgrade` to update the global `vp` binary, and use Vite+'s package management commands to update the local `vite-plus` package in a project.

## Overview

There are two parts to upgrading Vite+:

- The global `vp` command installed on your machine
- The local `vite-plus` package used by an individual project

You can upgrade both of them independently.

## Global `vp`

```bash
vp upgrade
```

## Local `vite-plus`

Update the project dependency with the package manager commands in Vite+:

```bash
vp update vite-plus
```

You can also use `vp add vite-plus@latest` if you want to move the dependency explicitly to the latest version.

### Updating Aliased Packages

Vite+ sets up npm aliases for its core packages during installation:

- `vite` is aliased to `npm:@voidzero-dev/vite-plus-core@latest`
- `vitest` is aliased to `npm:@voidzero-dev/vite-plus-test@latest`

`vp update vite-plus` does not re-resolve these aliases in the lockfile. To fully upgrade, update them separately:

```bash
vp update @voidzero-dev/vite-plus-core @voidzero-dev/vite-plus-test
```

Or update everything at once:

```bash
vp update vite-plus @voidzero-dev/vite-plus-core @voidzero-dev/vite-plus-test
```

You can verify with `vp outdated` that no Vite+ packages remain outdated.

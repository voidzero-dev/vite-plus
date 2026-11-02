# Package Management

Vite+ provides one command interface for npm, pnpm, Yarn, and Bun. `vp` detects the workspace's package manager, downloads the selected version when needed, and translates commands and supported options for it.

```bash
vp install
vp add react
vp add -D typescript
vp remove react
vp update
```

## Getting Started

- [Package Manager Selection](./package-manager-selection) explains detection, version ranges, and package-manager shims.
- [Global Packages](./global-packages) explains the separate Vite+ global package store.

## Commands

Dependency commands such as `vp add` run directly under `vp`, and other package-manager operations run under `vp pm`. Both forms are translated for the detected package manager in the same way. The `vp pm` prefix keeps names such as `pack` available for Vite+ commands. For example, `vp pm pack` creates a package tarball, but [`vp pack`](/guide/pack) builds a library.

## Native Arguments

Put native arguments after `--` to pass them through without translation:

```bash
vp install -- --registry=https://registry.npmjs.org
```

## Manage Dependencies

- [install](./install)
- [add](./add)
- [remove](./remove)
- [update](./update)
- [dedupe](./dedupe)
- [outdated](./outdated)
- [why](./why)
- [info](./info)
- [link](./link)
- [unlink](./unlink)
- [dlx](./dlx)

## `vp pm` Commands

- **Dependency management**
  - [ci](./ci)
  - [prune](./prune)
- **Inspection and diagnostics**
  - [list](./list)
  - [licenses](./licenses)
  - [search](./search)
  - [audit](./audit)
  - [fund](./fund)
  - [ping](./ping)
- **Local development**
  - [rebuild](./rebuild)
  - [approve-builds](./approve-builds)
  - [patch](./patch)
  - [patch-commit](./patch-commit)
- **Publishing**
  - [pack](./pack)
  - [version](./version)
  - [publish](./publish)
  - [stage](./stage)
  - [dist-tag](./dist-tag)
  - [deprecate](./deprecate)
- **Registry and accounts**
  - [login](./login)
  - [logout](./logout)
  - [whoami](./whoami)
  - [token](./token)
  - [owner](./owner)
  - [team](./team)
- **Configuration and maintenance**
  - [config](./config)
  - [pkg](./pkg)
  - [cache](./cache)

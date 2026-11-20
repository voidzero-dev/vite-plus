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

Core commands such as `vp add` run directly under `vp`. Forwarded Commands use `vp pm` to delegate operations to the detected package manager, translating commands and supported options just as core commands do. The `vp pm` prefix keeps names such as `pack` available for Vite+ commands. For example, `vp pm pack` creates a package tarball, but [`vp pack`](/guide/pack) builds a library.

## Native Arguments

Put native arguments after `--` to pass them through without translation:

```bash
vp install -- --registry=https://registry.npmjs.org
```

## Core Commands

- [install](./install)
- [add](./add)
- [remove](./remove)
- [update](./update)
- [dedupe](./dedupe)
- [outdated](./outdated)
- [list](./list)
- [why](./why)
- [info](./info)
- [link](./link)
- [unlink](./unlink)
- [rebuild](./rebuild)
- [dlx](./dlx)

## Forwarded Commands

- **Install and cleanup**
  - [pm ci](./ci)
  - [pm prune](./prune)
- **Inspection and diagnostics**
  - [pm licenses](./licenses)
  - [pm search](./search)
  - [pm audit](./audit)
  - [pm fund](./fund)
  - [pm ping](./ping)
- **Local development**
  - [pm approve-builds](./approve-builds)
  - [pm patch](./patch)
  - [pm patch-commit](./patch-commit)
- **Publishing**
  - [pm pack](./pack)
  - [pm version](./version)
  - [pm publish](./publish)
  - [pm stage](./stage)
  - [pm dist-tag](./dist-tag)
  - [pm deprecate](./deprecate)
- **Registry and accounts**
  - [pm login](./login)
  - [pm logout](./logout)
  - [pm whoami](./whoami)
  - [pm token](./token)
  - [pm owner](./owner)
  - [pm team](./team)
- **Configuration and maintenance**
  - [pm config](./config)
  - [pm pkg](./pkg)
  - [pm cache](./cache)

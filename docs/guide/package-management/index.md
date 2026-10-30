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

Common dependency operations are top-level commands such as `vp install`, `vp add`, and `vp why`. Additional operations use `vp pm`, such as `vp pm audit` and `vp pm publish`. The command reference groups them by purpose rather than by prefix.

```bash
vp install --help
vp pm --help
vp pm audit --help
```

Package-management commands are available in the [global CLI](/guide/global-cli) and the [project-local CLI](/guide/local-cli). The `vp list` and `vp rebuild` shortcuts require the global CLI. Use `vp pm list` and `vp pm rebuild` with either CLI. Global package operations with `-g` also require the global CLI.

## Native Arguments

Where supported, put additional native arguments after `--`:

```bash
vp install -- --registry=https://registry.npmjs.org
```

These arguments go directly to the native command without translation or Vite+'s unsupported-option checks. Their behavior depends on the package manager and version.

For `vp dlx`, arguments after the command belong to the executed program. See [Running Binaries](/guide/vpx).

## Command Reference

- **Dependency management**
  - [install](./install)
  - [add](./add)
  - [remove](./remove)
  - [update](./update)
  - [dedupe](./dedupe)
  - [ci](./ci)
  - [prune](./prune)
- **Inspection and diagnostics**
  - [list](./list)
  - [why](./why)
  - [info](./info)
  - [outdated](./outdated)
  - [licenses](./licenses)
  - [search](./search)
  - [audit](./audit)
  - [fund](./fund)
  - [ping](./ping)
- **Local development**
  - [link](./link)
  - [unlink](./unlink)
  - [rebuild](./rebuild)
  - [approve-builds](./approve-builds)
  - [patch](./patch)
  - [patch-commit](./patch-commit)
  - [dlx](./dlx)
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

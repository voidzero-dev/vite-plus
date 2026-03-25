# Getting Started

Vite+ is the unified toolchain and entry point for web development. It manages your runtime, package manager, and frontend toolchain in one place by combining [Vite](https://vite.dev/), [Vitest](https://vitest.dev/), [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), [Rolldown](https://rolldown.rs/), [tsdown](https://tsdown.dev/), and [Vite Task](https://github.com/voidzero-dev/vite-task).

Vite+ ships in two parts: `vp`, the global command-line tool, and `vite-plus`, the local package installed in each project. If you already have a Vite project, use [`vp migrate`](/guide/migrate) to migrate it to Vite+, or paste our [migration prompt](/guide/migrate#migration-prompt) into your coding agent.

## Install `vp`

### macOS / Linux

```bash
curl -fsSL https://vite.plus | bash
```

### Windows

```powershell
irm https://vite.plus/ps1 | iex
```

After installation, open a new shell and run:

```bash
vp help
```

::: info
Vite+ will manage your global Node.js runtime and package manager. If you'd like to opt out of this behavior, run `vp env off`. If you realize Vite+ is not for you, type `vp implode`, but please [share your feedback with us](https://discord.gg/cAnsqHh5PX).
:::

::: details Using a minor platform (CPU architecture, OS) ?

Prebuilt binaries are distributed for the following platforms (grouped by [Node.js v24 platform support tier](https://github.com/nodejs/node/blob/v24.x/BUILDING.md#platform-list)):

- Tier 1
  - Linux x64 glibc (`x86_64-unknown-linux-gnu`)
  - Linux arm64 glibc (`aarch64-unknown-linux-gnu`)
  - Windows x64 (`x86_64-pc-windows-msvc`)
  - macOS x64 (`x86_64-apple-darwin`)
  - macOS arm64 (`aarch64-apple-darwin`)
- Tier 2
  - Windows arm64 (`aarch64-pc-windows-msvc`)
- Experimental
  - Linux x64 musl (`x86_64-unknown-linux-musl`)
- Other
  - Linux arm64 musl (`aarch64-unknown-linux-musl`)

If a prebuilt binary is not available for your platform, installation will fail with an error.

On Alpine Linux (musl), you need to install `libstdc++` before using Vite+:

```sh
apk add libstdc++
```

This is required because the managed [unofficial-builds](https://unofficial-builds.nodejs.org/) Node.js runtime depends on the GNU C++ standard library.

:::

## Quick Start

Create a project, install dependencies, and use the default commands:

```bash
vp create # Create a new project
vp install # Install dependencies
vp dev # Start the dev server
vp check # Format, lint, type-check
vp test # Run JavaScript tests
vp build # Build for production
```

You can also just run `vp` on its own and use the interactive command line.

## Core Commands

Vite+ can handle the entire local frontend development cycle from starting a project, developing it, checking & testing, and building it for production.

### Start

- [`vp create`](/guide/create) creates new apps, packages, and monorepos.
- [`vp migrate`](/guide/migrate) moves existing projects onto Vite+.
- [`vp config`](/guide/commit-hooks) configures commit hooks and agent integration.
- [`vp staged`](/guide/commit-hooks) runs checks on staged files.
- [`vp install`](/guide/install) installs dependencies with the right package manager.
- [`vp env`](/guide/env) manages Node.js versions.

### Develop

- [`vp dev`](/guide/dev) starts the dev server powered by Vite.
- [`vp check`](/guide/check) runs format, lint, and type checks together.
- [`vp lint`](/guide/lint), [`vp fmt`](/guide/fmt), and [`vp test`](/guide/test) let you run those tools directly.

### Execute

- [`vp run`](/guide/run) runs tasks across workspaces with caching.
- [`vp cache`](/guide/cache) clears task cache entries.
- [`vpx`](/guide/vpx) runs binaries globally.
- [`vp exec`](/guide/vpx) runs local project binaries.
- [`vp dlx`](/guide/vpx) runs package binaries without adding them as dependencies.

### Build

- [`vp build`](/guide/build) builds apps.
- [`vp pack`](/guide/pack) builds libraries or standalone artifacts.
- [`vp preview`](/guide/build) previews the production build locally.

### Manage Dependencies

- [`vp add`](/guide/install), [`vp remove`](/guide/install), [`vp update`](/guide/install), [`vp dedupe`](/guide/install), [`vp outdated`](/guide/install), [`vp why`](/guide/install), and [`vp info`](/guide/install) wrap package-manager workflows.
- [`vp pm <command>`](/guide/install) calls other package manager commands directly.

### Maintain

- [`vp upgrade`](/guide/upgrade) updates the `vp` installation itself.
- [`vp implode`](/guide/implode) removes `vp` and related Vite+ data from your machine.

::: info
Vite+ ships with many predefined commands such as `vp build`, `vp test`, and `vp dev`. These commands are built in and cannot be changed. If you want to run a command from your `package.json` scripts, use `vp run <command>`.

[Learn more about `vp run`.](/guide/run)
:::

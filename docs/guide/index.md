# Getting Started

Vite+ is the unified toolchain and entry point for web development.

It brings together [Vite](https://vite.dev/), [Vitest](https://vitest.dev/), [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), [Rolldown](https://rolldown.rs/), [tsdown](https://tsdown.dev/), and [Vite Task](https://github.com/voidzero-dev/vite-task) in a single [`vite-plus` package](/guide/local-cli) for a blazing fast frontend toolchain.

Vite+ also ships a [global `vp` CLI](/guide/global-cli) that manages Node.js and package managers and makes Vite+ easier to use across projects. You can use either CLI independently, but we recommend [using them together](/guide/global-cli#use-both-clis-together).

If you already have a Vite project, run [`vp migrate`](/guide/migrate) to migrate it to Vite+, or give your coding agent our [migration prompt](/guide/migrate#migration-prompt).

Building with an AI assistant? View and copy a ready-made setup prompt:

<CopyPrompt />

## Install `vp` Globally

The commands below install the global `vp` CLI, which manages Node.js and package managers and makes `vp` available across projects. If you only need the frontend toolchain in a single project, you can install the [project-local CLI](/guide/local-cli#install) instead.

### macOS / Linux

```bash
curl -fsSL https://vite.plus | bash
```

### Windows

```powershell
irm https://vite.plus/ps1 | iex
```

Alternatively, download and run [`vp-setup.exe`](https://setup.viteplus.dev).

::: tip SmartScreen warning
The `vp-setup.exe` is not yet code-signed. Your browser may show a warning when downloading. Click **"..."** → **"Keep"** → **"Keep anyway"** to proceed. If Windows Defender SmartScreen blocks the file when you run it, click **"More info"** → **"Run anyway"**.
:::

The installer scripts and `vp-setup.exe` read [environment variables](/guide/global-cli#installation-variables) such as `VP_VERSION` and `VP_HOME`.

If you use Nushell with custom XDG directories, read the [Nushell startup requirements](/guide/global-cli#nushell-and-xdg-directories) before installing.

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

With the global CLI installed, create a project, install dependencies, and use the default commands:

```bash
vp create # Create a new project
vp install # Install dependencies
vp dev # Start the dev server
vp check # Format, lint, type-check
vp test # Run JavaScript tests
vp build # Build for production
```

You can also run `vp` on its own to open the interactive command line. In a local-only setup, run the same commands through your package manager, such as `pnpm exec vp check`.

## Core Commands

Vite+ covers the full frontend development cycle, from starting a project through development, checks, tests, and production builds. Most commands are available from both distributions; machine-level environment and self-management commands require the global CLI.

### Set Up a Project

- [`vp create`](/guide/create) creates new apps, packages, and monorepos.
- [`vp migrate`](/guide/migrate) moves existing projects onto Vite+.
- [`vp install`](/guide/install) installs dependencies with the right package manager.
- [`vp add`](/guide/install), [`vp remove`](/guide/install), [`vp update`](/guide/install), [`vp dedupe`](/guide/install), [`vp outdated`](/guide/install), [`vp list`](/guide/install), [`vp why`](/guide/install), and [`vp info`](/guide/install) cover the rest of the package-management workflow.
- [`vp link`](/guide/install), [`vp unlink`](/guide/install), [`vp rebuild`](/guide/install), and [`vp pm <command>`](/guide/install) provide lower-level package-manager operations.

### Project Toolchain

- [`vp check`](/guide/check) runs format, lint, and type checks together.
- [`vp lint`](/guide/lint) and [`vp fmt`](/guide/fmt) run the individual checks directly.
- [`vp test`](/guide/test) runs tests with Vitest.
- [`vp dev`](/guide/dev) starts the development server powered by Vite.
- [`vp build`](/guide/build) builds apps, and [`vp preview`](/guide/build) previews the production build locally.
- [`vp pack`](/guide/pack) builds libraries or standalone artifacts.
- [`vp toolchain`](/guide/upgrade#show-the-toolchain) shows the active project toolchain; use `--global` to inspect the global installation instead.
- [`vp run`](/guide/run) runs tasks across workspaces with caching.
- [`vp cache clean`](/guide/cache) clears task cache entries.
- [`vp exec`](/guide/vpx) runs local project binaries, while [`vp dlx`](/guide/vpx) and [`vpx`](/guide/vpx) download and run package binaries.
- [`vp config`](/guide/commit-hooks) installs the Git hook dispatcher and configures agent integration.
- [`vp hooks`](/guide/commit-hooks) manages the Git hook dispatcher, and [`vp staged`](/guide/commit-hooks) runs checks on staged files.
- [Monorepo Guide](/guide/monorepo) covers multi-package project structure and commands.

### Global CLI

- [`vp env`](/guide/env) manages Node.js and package-manager environments, and [`vp node`](/guide/env) runs scripts with the resolved environment.
- [`vp upgrade`](/guide/upgrade) updates the global `vp` installation itself.
- [`vp implode`](/guide/implode) removes the global `vp` installation and related Vite+ data from your machine.

### Workflow

- [IDE Integration](/guide/ide-integration), [CI](/guide/ci), and [Docker](/guide/docker) cover common development and deployment environments.

### Reference

- [Troubleshooting](/guide/troubleshooting) covers common command, configuration, and integration problems.

::: info
Vite+ ships with many predefined commands such as `vp build`, `vp test`, and `vp dev`. These commands are built-in and cannot be changed. If you want to run a command from your `package.json` scripts, use `vp run <command>` or `vpr <command>`.

[Learn more about `vp run`.](/guide/run)
:::

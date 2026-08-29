# Project-local CLI

The `vite-plus` package contains the project-local `vp` CLI and the integrated frontend toolchain. Install it as a development dependency when you want the toolchain version recorded in the project's manifest and lockfile, or when you do not want to install the standalone global CLI.

The local package includes Vite, Rolldown, Vitest, Oxlint, Oxfmt, tsdown, the Vite+ task runner, and package-manager commands. It requires an existing Node.js runtime and package manager.

## Install

::: code-group

```bash [pnpm]
pnpm add -D vite-plus
```

```bash [npm]
npm install -D vite-plus
```

```bash [Yarn]
yarn add -D vite-plus
```

```bash [Bun]
bun add -D vite-plus
```

:::

Run its binary through your package manager. For example:

```bash
pnpm exec vp help
pnpm exec vp check
```

Inside `package.json` scripts, `vp` resolves automatically from `node_modules/.bin`:

```json [package.json]
{
  "scripts": {
    "dev": "vp dev",
    "check": "vp check",
    "test": "vp test",
    "build": "vp build"
  }
}
```

The documentation uses bare `vp` commands for readability. Without the global CLI, prefix interactive commands with your package manager's local-binary executor, such as `pnpm exec`.

## What It Includes

The project-local CLI can be used independently for:

- [`vp dev`](/guide/dev), [`vp build`](/guide/build), and [`vp preview`](/guide/build) with Vite and Rolldown
- [`vp check`](/guide/check), [`vp lint`](/guide/lint), and [`vp fmt`](/guide/fmt) with Oxc
- [`vp test`](/guide/test) with Vitest
- [`vp pack`](/guide/pack) with tsdown
- [`vp toolchain`](/guide/upgrade#show-the-toolchain) for inspecting the versions bundled with the project-local package
- [`vp run`](/guide/run) and task caching across workspaces
- [package-manager commands](/guide/install) using the Node.js runtime already active in your shell
- [`vp create`](/guide/create), [`vp migrate`](/guide/migrate), and project configuration commands

The local package cannot manage the machine-level Vite+ installation. The `vp env`, `vp upgrade`, and `vp implode` commands require the [global CLI](/guide/global-cli). Upgrade or remove a local-only installation through your package manager.

## Adopt Vite+ in an Existing Project

For an existing Vite project, use [`vp migrate`](/guide/migrate) instead of manually replacing each tool and configuration file. From a local-only installation, run:

```bash
pnpm exec vp migrate
```

For a new project, [`vp create`](/guide/create) can scaffold an application, library, or monorepo. If you prefer to assemble the project yourself, import configuration APIs from `vite-plus`:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
  },
});
```

## Add the Global CLI Later

You can install the global CLI at any time without changing the project's dependency. Once installed, bare `vp` commands use the local `vite-plus` version for project commands and reserve machine-level commands such as `vp env` for the global binary.

See [Use Both CLIs Together](/guide/global-cli#use-both-clis-together) for the selection rules.

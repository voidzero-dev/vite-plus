# Project-local CLI

Different from [global `vp` cli](/guide/global-cli), the `vite-plus` is a npm package which contains the project-local `vp` CLI and the integrated frontend toolchain. Install it as a development dependency when you want the toolchain version recorded in the project's manifest and lockfile, or when you do not want to install the standalone global CLI.

The local package includes Vite, Rolldown, Vitest, Oxlint, Oxfmt, tsdown, the Vite+ task runner, and package-manager commands. It requires an existing Node.js runtime and package manager.

## Install

For most of use cases, we recommend to use Vite+ cli to install in a project or create a new project. Learn more in [Creating a Project](/guide/create) and [Migrate to Vite+](/guide/migrate).

::: code-group

```bash [pnpm]
pnpm dlx --package=vite-plus vp create
```

```bash [npm]
npx --package=vite-plus vp create
```

```bash [Yarn]
yarn dlx --package vite-plus vp create
```

```bash [Bun]
bunx --package vite-plus vp create
```

:::

Run its binary through your package manager. For example:

```bash
./node_modules/.bin/vp migrate --help
./node_modules/.bin/vp check
```

The documentation uses bare `vp` commands for readability. Without the global CLI, prefix interactive commands with your package manager's local-binary executor, such as `pnpm exec`.

### Manual Installation

If you are manually migrating a project to Vite+, install these dev dependencies first:

```bash
vp install -D vite-plus
```

You need to add overrides to your package manager so that other packages resolve the Vite+ versions: alias `vite` to `@voidzero-dev/vite-plus-core`, and pin `vitest` to the version Vite+ bundles (run `vp --version`) so the whole project shares a single Vitest copy with `vp test`. Without the `vitest` pin, a dependency or workspace package can pull a different Vitest than the bundled runner, splitting Vitest's internals (mocks, `expect`, runner state):

::: code-group

```yaml [pnpm-workspace.yaml]
overrides:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: 4.1.11
```

```json [npm / Bun package.json]
"overrides": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "4.1.11"
}
```

```json [Yarn package.json]
"resolutions": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "4.1.11"
}
```

:::

::: details Why are these settings needed?

Dependencies and plugins can import `vite` or `vitest` directly, even when your own code imports from `vite-plus`. These overrides align their dependencies with the toolchain Vite+ uses:

- The `vite` alias directs those imports to Vite+'s core package. Separate Vite instances can break runtime identity checks: [issue #1391](https://github.com/voidzero-dev/vite-plus/issues/1391) reported TanStack Start returning 404s because an `instanceof` check crossed two copies. [PR #2617](https://github.com/voidzero-dev/vite-plus/pull/2617) addresses the CLI side by sharing Vite through the same alias.
- The exact `vitest` pin keeps dependencies and `vp test` on the same Vitest version, avoiding separate mocks, `expect` instances, and runner state. [PR #2365](https://github.com/voidzero-dev/vite-plus/pull/2365) documents this requirement for manual installation.

Keep the core alias aligned with your installed `vite-plus` version and update the Vitest pin to match its bundled version when upgrading. [Issue #2356](https://github.com/voidzero-dev/vite-plus/issues/2356) describes how dependency bots can update these packages independently and leave incompatible versions installed together.

:::

## Best Practices

We recommend using the [global CLI](/guide/global-cli) together with the project-local CLI. The global CLI makes `vp` available directly in your terminal and delegates development commands such as `vp dev`, `vp build`, and `vp test` to the project's installed `vite-plus` package. This gives you convenient access to the toolchain while keeping its version controlled by the project. You can also use only the project-local CLI if you prefer.

For open-source projects or any project with collaborators, we recommend adding `package.json` scripts that call `vp`, whether you use both CLIs or only the project-local CLI. Inside scripts, `vp` resolves automatically from `node_modules/.bin`:

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

After installing the project's dependencies, contributors can run these scripts through their package manager, such as `pnpm run dev` or `npm run dev`, without being required to install the global CLI.

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

## Add the Global CLI Later

You can install the global CLI at any time without changing the project's dependency. Commands such as `vp dev`, `vp build`, and `vp test` will continue to use the project's installed `vite-plus` version.

See [Use Both CLIs Together](/guide/global-cli#use-both-clis-together) for the selection rules.

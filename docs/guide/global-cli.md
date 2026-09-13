# Global CLI

The global CLI is a standalone `vp` binary for machine-level runtime and package management. It includes a Vite+ toolchain, does not require Node.js to be installed first, and can be used without adding `vite-plus` to a project.

Choose the global CLI when you want one command available across projects for any combination of:

- managing Node.js and package-manager versions
- selecting and downloading package managers
- installing dependencies and running package binaries
- running `package.json` scripts and cached workspace tasks
- using the Vite+ frontend toolchain without pinning it in every project

Installing the global CLI does not require you to adopt the project-local package. You can use it only for runtime management, package management, and the task runner if that is all you need.

## Install

::: code-group

```bash [macOS / Linux]
curl -fsSL https://vite.plus | bash
```

```powershell [Windows]
irm https://vite.plus/ps1 | iex
```

:::

On Windows, you can instead download and run [`vp-setup.exe`](https://setup.viteplus.dev).

After installation, open a new shell and run:

```bash
vp help
```

When you enable environment management during installation, Vite+ records managed mode for Node.js and the npm, pnpm, Yarn, and Bun shims. Run `vp env off` to prefer system tools, or scope the change with `vp env off node` or `vp env off pm`.

### Installer Options

The installers accept environment variables for the less common cases where the defaults do not fit:

| Variable              | Purpose                                                          |
| --------------------- | ---------------------------------------------------------------- |
| `VP_VERSION`          | Install a specific Vite+ version instead of `latest`             |
| `VP_HOME`             | Put the installation and managed data under one custom directory |
| `NPM_CONFIG_REGISTRY` | Download Vite+ packages from a custom npm registry               |
| `VP_NODE_MANAGER=no`  | Skip environment manager setup                                   |
| `VP_PR_VERSION`       | Install a preview build by pull request number or commit SHA     |

Set a variable for the installer command:

::: code-group

```bash [macOS / Linux]
curl -fsSL https://vite.plus | VP_VERSION=1.2.3 bash
```

```powershell [Windows]
$env:VP_VERSION = "1.2.3"; irm https://vite.plus/ps1 | iex
```

:::

Command-line options passed to `vp-setup.exe` take precedence over environment variables. See [Installer Environment Variables](/guide/installer-env-vars) for the full reference, including directory overrides, runtime settings, and download configuration.

## Use It Without a Local Package

The global installation is enough for runtime, package-manager, and task-runner workflows:

```bash
vp env pin lts       # Pin and install Node.js for this project
vp install           # Use the package manager declared by the project
vp run build         # Run a package.json script or configured task
vp dlx create-vite   # Download and run a package binary
```

You do not need a local `vite-plus` dependency to run existing `package.json` scripts. Add the [project-local CLI](/guide/local-cli) when you want the frontend toolchain version recorded in the project's manifest and lockfile.

## Use Both CLIs Together

The global command is also the entry point for projects that install `vite-plus` locally. It resolves the toolchain from the directory where you run it:

| Current project                             | What `vp` uses for project commands                |
| ------------------------------------------- | -------------------------------------------------- |
| Has a runnable local `vite-plus` dependency | The project-local CLI and its pinned toolchain     |
| Has no local `vite-plus` dependency         | The toolchain bundled with the global installation |

Global-only commands such as `vp env`, `vp upgrade`, and `vp implode` remain owned by the standalone installation.

This gives you a stable command on `PATH` while each adopted project can pin and upgrade its development toolchain independently.

## Next Steps

- [Environment](/guide/env) covers Node.js and package-manager selection, pinning, shims, and managed installations.
- [Package Management](/guide/install) covers pnpm, npm, Yarn, and Bun workflows.
- [Run](/guide/run) covers package scripts and cached workspace tasks.
- [Upgrading Vite+](/guide/upgrade) explains independent global and project-local upgrades.
- [Removing Vite+](/guide/implode) removes the global binary and its managed data.

::: details Platform support

Prebuilt binaries are distributed for:

- Linux x64 and arm64 with glibc
- Windows x64 and arm64
- macOS x64 and arm64
- Linux x64 and arm64 with musl

If a prebuilt binary is not available for your platform, installation fails with an error. On Alpine Linux, install `libstdc++` before using the managed [unofficial Node.js builds](https://unofficial-builds.nodejs.org/):

```sh
apk add libstdc++
```

:::

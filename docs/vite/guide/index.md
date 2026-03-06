# Getting Started

## Overview

Vite+ is a unified toolchain for modern web development. It combines [Vite](https://vite.dev/), [Vitest](https://vitest.dev/), [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), [Rolldown](https://rolldown.rs/), [tsdown](https://tsdown.dev/), and [Vite Task](https://github.com/voidzero-dev/vite-task) into a single CLI:

- **Runtime Management**: Manage Node.js globally and per project with `vp env`
- **Package Management**: Install and manage dependencies with `vp install` and package-manager wrappers
- **Dev Server**: Run Vite's native ESM dev server with instant HMR via `vp dev`
- **Code Health**: Run type checks, linting, and formatting with `vp check`
- **Testing**: Run tests through bundled Vitest with `vp test`
- **Build & Pack**: Build apps with `vp build` and build libraries or standalone app binaries with `vp pack`
- **Task Runner**: Run monorepo tasks with caching and dependency-aware scheduling via `vp run`
- **Scaffolding & Migration**: Create new projects and migrate existing ones with `vp create` and `vp migrate`

All in a single, cohesive tool designed for scale, speed, and developer sanity.
Vite+ is fully open-source under the MIT license.

## Installation

Install Vite+ globally as `vp`:

For Linux or macOS:

```bash
curl -fsSL https://viteplus.dev/install.sh | bash
```

For Windows:

```bash
irm https://viteplus.dev/install.ps1 | iex
```

::: details Supported platforms

Prebuilt binaries are distributed for the following platforms (grouped by [Node.js v24 platform support tier](https://github.com/nodejs/node/blob/v24.x/BUILDING.md#platform-list)):

- Tier 1
  - Linux x64 glibc (`x86_64-unknown-linux-gnu`)
  - Linux arm64 glibc (`aarch64-unknown-linux-gnu`)
  - Windows x64 (`x86_64-pc-windows-msvc`)
  - macOS x64 (`x86_64-apple-darwin`)
  - macOS arm64 (`aarch64-apple-darwin`)
- Tier 2
  - Windows arm64 (`aarch64-pc-windows-msvc`)

:::

## Node.js Version Manager

Vite+ includes a built-in Node.js version manager. During installation, you can opt-in to let Vite+ manage your Node.js versions.

```bash
vp env pin 22.12.0        # Pin version in .node-version
vp env default lts        # Set global default
vp env list               # Show available versions
vp env doctor             # Diagnose issues
vp env help               # Show all commands
```

## Scaffolding Your First Vite+ Project

Create a Vite+ project:

```bash
vp create
```

Follow the prompts to select your preferred framework and configuration.

## CLI Workflows

`vp help` organizes commands by workflow:

```bash
# Start
vp create           # Scaffold a new project
vp migrate          # Migrate an existing project
vp install          # Install dependencies
vp env              # Manage Node.js versions

# Develop
vp dev              # Start dev server
vp check            # Format, lint, and type-check
vp test             # Run tests

# Execute
vp run              # Run monorepo tasks
vp exec             # Run local binaries
vp dlx              # Run remote/local package binaries

# Build
vp build            # Build applications
vp pack             # Build libraries
vp preview          # Preview production build
```

## Monorepo Task Execution

Vite+ includes a powerful task runner for managing tasks across monorepo packages:

### Run tasks recursively

```bash
vp run build -r              # Build all packages with topological ordering
vp run test -r               # Test all packages
```

### Run tasks for specific packages

```bash
vp run app#build web#build   # Build specific packages
vp run @scope/*#test         # Test all packages matching pattern
```

### Current package

```bash
vp dev                       # Run dev script in current package
```

## Task Dependencies

Tasks automatically respect dependencies:

1. **Explicit dependencies** - Defined in `vite-task.json`:

```json
{
  "tasks": {
    "test": {
      "command": "jest",
      "dependsOn": ["build", "lint"]
    }
  }
}
```

2. **Implicit dependencies** - Based on `package.json` relationships when using `--topological` (default for `-r`):
   - If package A depends on package B, then `A#build` automatically depends on `B#build`

Disable topological ordering:

```bash
vp run build -r --no-topological
```

## Intelligent Caching

Vite+ caches task outputs to speed up repeated builds:

- Automatically detects when inputs change
- Skips tasks when outputs are cached
- Shares cache across team members (when configured)

View cache operations:

```bash
vp run build -r --debug
```

## CI/CD

Use the official [`setup-vp`](https://github.com/voidzero-dev/setup-vp) GitHub Action to install Vite+ in CI:

### Basic usage

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: voidzero-dev/setup-vp@v1
    with:
      node-version: '22'
      cache: true
  - run: vp run build -r
  - run: vp run test -r
```

### Matrix testing

```yaml
jobs:
  test:
    strategy:
      matrix:
        node-version: ['20', '22', '24']
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: voidzero-dev/setup-vp@v1
        with:
          node-version: ${{ matrix.node-version }}
          cache: true
      - run: vp run test -r
```

See the [setup-vp README](https://github.com/voidzero-dev/setup-vp) for all options.

## Next Steps

- Learn more about [task configuration](./task/getting-started)
- Explore [caching strategies](./caching)
- Set up [monorepo workspaces](./monorepo)
- Customize [Vite+ configuration](../../config/)

## Community & Support

Get help and stay updated:

- [GitHub Issues](https://github.com/voidzero-dev/vite-plus/issues)
- [GitHub Discussions](https://github.com/voidzero-dev/vite-plus/discussions)

---

::: tip Requirements
Vite+ requires Node.js 20.19+, 22.12+ or 24.12+
:::

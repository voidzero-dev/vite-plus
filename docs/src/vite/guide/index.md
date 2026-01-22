# Getting Started

## Overview

Vite+ is a unified toolchain for modern web development that extends Vite with powerful monorepo capabilities. It combines:

- **Dev Server**: Vite's blazing-fast development experience with native ES modules and instant HMR
- **Build Tool**: Optimized production builds powered by Rolldown
- **Task Runner**: Intelligent monorepo task execution with caching and dependency resolution
- **Testing**: Built-in test runner with workspace support
- **Linting**: Integrated oxlint for fast code quality checks
- **Formatting**: Integrated oxfmt for consistent code formatting
- **Code Generation**: Scaffolding for new projects and monorepo workspaces
- **Dependency Management**: Integrated dependency management with pnpm, yarn, npm and bun(coming soon)

All in a single, cohesive tool designed for scale, speed, and developer sanity.

## Installation

### Global CLI

```bash
npm install -g vite-plus-cli
```

## Scaffolding Your First Vite+ Project

Create a Vite+ project:

```bash
vite new
```

Follow the prompts to select your preferred framework and configuration.

## Core Commands

Vite+ provides built-in commands that work seamlessly in both single-package and monorepo setups:

```bash
# Development
vite dev              # Start dev server

# Build
vite build            # Build for production

# Test
vite test             # Run tests

# Lint
vite lint             # Lint code with oxlint
```

## Monorepo Task Execution

Vite+ includes a powerful task runner for managing tasks across monorepo packages:

### Run tasks recursively

```bash
vite run build -r              # Build all packages with topological ordering
vite run test -r               # Test all packages
```

### Run tasks for specific packages

```bash
vite run app#build web#build   # Build specific packages
vite run @scope/*#test         # Test all packages matching pattern
```

### Current package

```bash
vite dev                       # Run dev script in current package
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
vite run build -r --no-topological
```

## Intelligent Caching

Vite+ caches task outputs to speed up repeated builds:

- Automatically detects when inputs change
- Skips tasks when outputs are cached
- Shares cache across team members (when configured)

View cache operations:

```bash
vite run build -r --debug
```

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

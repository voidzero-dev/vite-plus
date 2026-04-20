<picture>
  <source media="(prefers-color-scheme: dark)" srcset="/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="/logo.svg">
  <img alt="Vite+" src="/logo.svg">
</picture>

**The Unified Toolchain for the Web**
_runtime and package management, create, dev, check, test, build, pack, and monorepo task caching in a single dependency_

---

Vite+ is the unified entry point for local web development. It combines [Vite](https://vite.dev/), [Vitest](https://vitest.dev/), [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), [Rolldown](https://rolldown.rs/), [tsdown](https://tsdown.dev/), and [Vite Task](https://github.com/voidzero-dev/vite-task) into one zero-config toolchain that also manages runtime and package manager workflows:

- **`vp env`:** Manage Node.js globally and per project
- **`vp install`:** Install dependencies with automatic package manager detection
- **`vp dev`:** Run Vite's fast native ESM dev server with instant HMR
- **`vp check`:** Run formatting, linting, and type checks in one command
- **`vp test`:** Run tests through bundled Vitest
- **`vp build`:** Build applications for production with Vite + Rolldown
- **`vp run`:** Execute monorepo tasks with caching and dependency-aware scheduling
- **`vp pack`:** Build libraries for npm publishing or standalone app binaries
- **`vp create` / `vp migrate`:** Scaffold new projects and migrate existing ones

All of this is configured from your project root and works across Vite's framework ecosystem.
Vite+ is fully open-source under the MIT license.

## Getting Started

Install Vite+ globally as `vp`:

For Linux or macOS:

```bash
curl -fsSL https://vite.plus | bash
```

For Windows:

```bash
irm https://viteplus.dev/install.ps1 | iex
```

`vp` handles the full development lifecycle such as package management, development servers, linting, formatting, testing and building for production.

## Configuring Vite+

Vite+ can be configured using a single `vite.config.ts` at the root of your project:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  // Standard Vite configuration for dev/build/preview.
  plugins: [],

  // Vitest configuration.
  test: {
    include: ['src/**/*.test.ts'],
  },

  // Oxlint configuration.
  lint: {
    ignorePatterns: ['dist/**'],
  },

  // Oxfmt configuration.
  fmt: {
    semi: true,
    singleQuote: true,
  },

  // Vite Task configuration.
  run: {
    tasks: {
      'generate:icons': {
        command: 'node scripts/generate-icons.js',
        envs: ['ICON_THEME'],
      },
    },
  },

  // `vp staged` configuration.
  staged: {
    '*': 'vp check --fix',
  },
});
```

This lets you keep the configuration for your development server, build, test, lint, format, task runner, and staged-file workflow in one place with type-safe config and shared defaults.

Use `vp migrate` to migrate to Vite+. It merges tool-specific config files such as `.oxlintrc*`, `.oxfmtrc*`, and lint-staged config into `vite.config.ts`.

### CLI Workflows (`vp help`)

#### Start

- **create** - Create a new project from a template
- **migrate** - Migrate an existing project to Vite+
- **config** - Configure hooks and agent integration
- **staged** - Run linters on staged files
- **install** (`i`) - Install dependencies
- **env** - Manage Node.js versions

#### Develop

- **dev** - Run the development server
- **check** - Run format, lint, and type checks
- **lint** - Lint code
- **fmt** - Format code
- **test** - Run tests

#### Execute

- **run** - Run monorepo tasks
- **exec** - Execute a command from local `node_modules/.bin`
- **dlx** - Execute a package binary without installing it as a dependency
- **cache** - Manage the task cache

#### Build

- **build** - Build for production
- **pack** - Build libraries
- **preview** - Preview production build

#### Manage Dependencies

Vite+ automatically wraps your package manager (pnpm, npm, or Yarn) based on `packageManager` and lockfiles:

- **add** - Add packages to dependencies
- **remove** (`rm`, `un`, `uninstall`) - Remove packages from dependencies
- **update** (`up`) - Update packages to latest versions
- **dedupe** - Deduplicate dependencies
- **outdated** - Check outdated packages
- **list** (`ls`) - List installed packages
- **why** (`explain`) - Show why a package is installed
- **info** (`view`, `show`) - View package metadata from the registry
- **link** (`ln`) / **unlink** - Manage local package links
- **pm** - Forward a command to the package manager

#### Maintain

- **upgrade** - Update `vp` itself to the latest version
- **implode** - Remove `vp` and all related data

### Scaffolding your first Vite+ project

Use `vp create` to create a new project:

```bash
vp create
```

You can run `vp create` inside of a project to add new apps or libraries to your project.

### Migrating an existing project

You can migrate an existing project to Vite+:

```bash
vp migrate
```

### GitHub Actions

Use the official [`setup-vp`](https://github.com/voidzero-dev/setup-vp) action to install Vite+ in GitHub Actions:

```yaml
- uses: voidzero-dev/setup-vp@v1
  with:
    node-version: '22'
    cache: true
```

#### Manual Installation & Migration

If you are manually migrating a project to Vite+, install these dev dependencies first:

```bash
npm install -D vite-plus @voidzero-dev/vite-plus-core@latest
```

You need to add overrides to your package manager for `vite` and `vitest` so that other packages depending on Vite and Vitest will use the Vite+ versions:

```json
"overrides": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
}
```

If you are using `pnpm`, add this to your `pnpm-workspace.yaml`:

```yaml
overrides:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
```

Or, if you are using Yarn:

```json
"resolutions": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
}
```

## Sponsors

Thanks to [namespace.so](https://namespace.so) for powering our CI/CD pipelines with fast, free macOS and Linux runners.

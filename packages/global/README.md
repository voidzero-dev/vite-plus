# VITE+(⚡︎)

**The Unified Toolchain for the Web**
_dev, build, test, lint, format, monorepo caching & more in a single dependency, built for scale, speed, and sanity_

---

Vite+ combines [Vite](https://vite.dev/), [Vitest](https://vitest.dev/), [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), [tsdown](https://tsdown.dev/) and [Rolldown](https://rolldown.rs/) as a single zero-config toolchain:

- **Dev Server:** Powered by Vite's fast development experience with native ES modules and instant HMR
- **Build Tool:** Optimized production builds using Rolldown and Oxc
- **Testing:** Seamless Vitest integration with fast feedback loops
- **Linting:** Ships with Oxlint for quick code quality checks
- **Task Runner:** Monorepo task execution with automated caching and dependency resolution
- **Package Management:** Vite+ wraps package managers to provide a unified interface

Vite+ is built to scale with your codebase while reducing your devtools to a single dependency.

## Getting Started

Vite+ requires Node.js 22+. Install `vite-plus-cli` globally as `vite`:

```bash
npm install -g vite-plus-cli
```

`vite` handles the full development lifecycle such as package management, development servers, linting, formatting, testing and building for production.

### Vite+ Commands

- **dev** - Run the development server
- **build** - Build for production
- **lint** - Lint code
- **test** - Run tests
- **fmt** - Format code
- **lib** - Build library
- **migrate** - Migrate an existing project to Vite+
- **new** - Create a new monorepo package (in-project) or a new project (global)
- **run** - Run tasks from `package.json` scripts

### Package Manager Commands

Vite+ automatically detects and wraps the underlying package manager such as pnpm, npm, or Yarn through the `packageManager` field in `package.json` or package manager-specific lockfiles.

- **install** - Install all dependencies, or add packages if package names are provided
- **add** - Add packages to dependencies
- **remove** - Remove packages from dependencies
- **dlx** - Execute a package binary without installing it as a dependency
- **info** - View package information from the registry, including latest versions
- **link** - Link packages for local development
- **outdated** - Check for outdated packages
- **pm** - Forward a command to the package manager
- **unlink** - Unlink packages
- **update** - Update packages to their latest versions
- **why** - Show why a package is installed

### Scaffolding your first Vite+ project

Use `vp new` to create a new project:

```bash
vp new
```

You can run `vp new` inside of a project to add new apps or libraries to your project.

### Migrating an existing project

You can migrate an existing project to Vite+:

```bash
vp migrate
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

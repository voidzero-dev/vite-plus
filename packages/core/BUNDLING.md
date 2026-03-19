# Core Package Bundling Architecture

This document explains how `@voidzero-dev/vite-plus-core` bundles multiple upstream projects into a single unified package.

## Overview

The core package uses a **multi-project bundling strategy** that combines 5 upstream projects:

| Project                 | Source Location                 | Purpose                   |
| ----------------------- | ------------------------------- | ------------------------- |
| `@rolldown/pluginutils` | `rolldown/packages/pluginutils` | Rolldown plugin utilities |
| `rolldown`              | `rolldown/packages/rolldown`    | Rolldown bundler          |
| `vite`                  | `vite/packages/vite`            | Vite v8 beta              |
| `tsdown`                | `node_modules/tsdown`           | TypeScript build tool     |
| `vitepress`             | `node_modules/vitepress`        | Documentation tool        |

This approach enables users to access Vite, Rolldown, and related tools through a single package with consistent module specifier rewrites.

---

## Build Steps

The build process executes 6 steps in sequence:

### Step 1: Bundle Rolldown Pluginutils (`bundleRolldownPluginutils`)

**Action**: Copies pre-built dist directory.

```typescript
await cp(join(rolldownPluginUtilsDir, 'dist'), join(projectDir, 'dist', 'pluginutils'), {
  recursive: true,
});
```

**Input**: `rolldown/packages/pluginutils/dist/`
**Output**: `dist/pluginutils/`

### Step 2: Bundle Rolldown (`bundleRolldown`)

**Action**: Copies dist directory and rewrites module specifiers.

**Transformations**:

- `@rolldown/pluginutils` → `@voidzero-dev/vite-plus-core/rolldown/pluginutils`
- `rolldown/*` → `@voidzero-dev/vite-plus-core/rolldown/*`
- In release builds: `@rolldown/binding-*` → `vite-plus/binding`

**Input**: `rolldown/packages/rolldown/dist/`
**Output**: `dist/rolldown/`

### Step 3: Build Vite (`buildVite`)

**Action**: Full Rolldown build with custom transforms.

This is the most complex step, using the upstream `vite-rolldown.config` with modifications:

1. **Filter externals** - Bundles `picomatch`, `tinyglobby`, `fdir`, `rolldown`, `yaml` instead of keeping them external
2. **Add RewriteImportsPlugin** - Rewrites vite/rolldown imports at build time
3. **Rewrite static paths** - Fixes `VITE_PACKAGE_DIR`, `CLIENT_ENTRY`, `ENV_ENTRY` constants
4. **Copy additional files** - `misc/`, `.d.ts` files, `types/`, `client.d.ts`

**Input**: `vite/packages/vite/`
**Output**: `dist/vite/`

### Step 4: Bundle Tsdown (`bundleTsdown`)

**Action**: Re-bundles tsdown with CJS dependency handling.

**Process**:

1. Bundle `tsdown/dist/run.mjs` and `tsdown/dist/index.mjs` using Rolldown
2. Detect third-party CJS modules using `find-create-require.ts`
3. Bundle detected CJS dependencies using `build-cjs-deps.ts`
4. Bundle type declarations using `rolldown-plugin-dts`

**Input**: `node_modules/tsdown/dist/`
**Output**: `dist/tsdown/`

### Step 5: Bundle Vitepress (`bundleVitepress`)

**Action**: Copies dist directory and rewrites vite imports.

**Transformations**:

- `vite` → `@voidzero-dev/vite-plus-core/vite`

**Input**: `node_modules/vitepress/`
**Output**: `dist/vitepress/`

### Step 6: Merge Package.json (`mergePackageJson`)

**Action**: Merges metadata from upstream packages and records bundled versions.

**Updates**:

- `peerDependencies` - Merged from tsdown and vite
- `peerDependenciesMeta` - Merged from tsdown and vite
- `bundledVersions` - Records vite, rolldown, and tsdown versions

---

## Module Specifier Rewriting System

The build uses two complementary rewriting mechanisms:

### Build-Time Rewriting (RewriteImportsPlugin)

Located in `build-support/rewrite-imports.ts`, this Rolldown plugin rewrites imports during bundling:

```typescript
export const RewriteImportsPlugin: Plugin = {
  name: 'rewrite-imports-for-vite-plus',
  resolveId: {
    order: 'pre',
    handler(id: string) {
      if (id.startsWith('vite/')) {
        return { id: id.replace(/^vite\//, `${pkgJson.name}/`), external: true };
      }
      if (id === 'rolldown') {
        return { id: `${pkgJson.name}/rolldown`, external: true };
      }
      if (id.startsWith('rolldown/')) {
        return { id: id.replace(/^rolldown\//, `${pkgJson.name}/rolldown/`), external: true };
      }
    },
  },
};
```

### Post-Build Rewriting (AST-grep)

Located in `build-support/rewrite-module-specifiers.ts`, this utility rewrites specifiers in already-built files using AST-grep:

| Original Import           | Rewritten Import                                      |
| ------------------------- | ----------------------------------------------------- |
| `vite`                    | `@voidzero-dev/vite-plus-core`                        |
| `vite/*`                  | `@voidzero-dev/vite-plus-core/*`                      |
| `rolldown`                | `@voidzero-dev/vite-plus-core/rolldown`               |
| `rolldown/*`              | `@voidzero-dev/vite-plus-core/rolldown/*`             |
| `@rolldown/pluginutils`   | `@voidzero-dev/vite-plus-core/rolldown/pluginutils`   |
| `@rolldown/pluginutils/*` | `@voidzero-dev/vite-plus-core/rolldown/pluginutils/*` |

### Release Build: Native Binding Rewriting

During release builds (`RELEASE_BUILD=1`), an additional critical transformation occurs for Rolldown's native bindings:

```typescript
// In bundleRolldown()
if (process.env.RELEASE_BUILD) {
  // @rolldown/binding-darwin-arm64 → vite-plus/binding
  source = source.replace(/@rolldown\/binding-([a-z0-9-]+)/g, 'vite-plus/binding');
  // Sync version strings
  source = source.replaceAll(`${rolldownBindingVersion}`, pkgJson.version);
}
```

**Platform-specific binding rewrites**:

| Original Import                     | Rewritten Import    |
| ----------------------------------- | ------------------- |
| `@rolldown/binding-darwin-arm64`    | `vite-plus/binding` |
| `@rolldown/binding-darwin-x64`      | `vite-plus/binding` |
| `@rolldown/binding-linux-arm64-gnu` | `vite-plus/binding` |
| `@rolldown/binding-linux-x64-gnu`   | `vite-plus/binding` |
| `@rolldown/binding-win32-x64-msvc`  | `vite-plus/binding` |

**Why this matters**:

1. **Self-contained distribution** - Users don't need to install separate `@rolldown/binding-*` packages
2. **Version alignment** - The rolldown binding version is synced to the vite-plus version
3. **Single native module** - The `vite-plus/binding` export points to the CLI's compiled `.node` file which includes `rolldown_binding` when built with `RELEASE_BUILD=1`

**Resolution chain**:

```
User code imports '@voidzero-dev/vite-plus-core/rolldown'
  → dist/rolldown/index.mjs
    → imports 'vite-plus/binding' (rewritten from @rolldown/binding-*)
      → vite-plus CLI package ./binding export
        → binding/vite-plus.darwin-arm64.node (contains rolldown_binding)
```

See [CLI Package Bundling](../cli/BUNDLING.md#rolldown-native-binding-integration) for details on how the CLI compiles rolldown bindings.

---

## CJS Dependency Handling

Tsdown uses `createRequire()` to load some CommonJS dependencies. These are detected and bundled specially:

### Detection (`find-create-require.ts`)

Uses `oxc-parser` to find patterns like:

```javascript
// Pattern 1: Static import
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
require('some-cjs-package');

// Pattern 2: Global module
const require = globalThis.process.getBuiltinModule('module').createRequire(import.meta.url);
require('some-cjs-package');
```

### Bundling (`build-cjs-deps.ts`)

Creates CJS entry files and bundles them with Rolldown:

```typescript
// Creates: npm_entry_some_cjs_package.cjs
module.exports = require('some-cjs-package');
```

The original `require("some-cjs-package")` calls are rewritten to `require("./npm_entry_some_cjs_package.cjs")`.

---

## Output Structure

```
dist/
├── pluginutils/           # @rolldown/pluginutils
│   ├── index.js
│   ├── index.d.ts
│   └── filter/
├── rolldown/              # Rolldown bundler
│   ├── index.mjs
│   ├── index.d.mts
│   ├── config.mjs
│   ├── experimental-index.mjs
│   ├── filter-index.mjs
│   ├── parallel-plugin.mjs
│   ├── parse-ast-index.mjs
│   ├── plugins-index.mjs
│   └── ...
├── vite/                  # Vite
│   ├── node/
│   │   ├── index.js
│   │   ├── index.d.ts
│   │   ├── internal.js
│   │   ├── module-runner.js
│   │   └── chunks/
│   ├── client/
│   │   ├── client.mjs
│   │   └── env.mjs
│   ├── misc/
│   ├── types/
│   └── client.d.ts
├── tsdown/                # TypeScript build tool
│   ├── index.js
│   ├── index-types.d.ts
│   ├── run.js
│   └── npm_entry_*.cjs    # Bundled CJS deps
└── vitepress/             # Documentation tool
    ├── dist/
    ├── types/
    ├── client.d.ts
    ├── theme.d.ts
    └── theme-without-fonts.d.ts
```

---

## Package Exports

| Export Path                     | Points To                                | Description             |
| ------------------------------- | ---------------------------------------- | ----------------------- |
| `.`                             | `./dist/vite/node/index.js`              | Vite main entry         |
| `./client`                      | types: `./dist/vite/client.d.ts`         | Client ambient types    |
| `./dist/client/*`               | `./dist/vite/client/*`                   | Client runtime files    |
| `./internal`                    | `./dist/vite/node/internal.js`           | Internal Vite APIs      |
| `./lib`                         | `./dist/tsdown/index.js`                 | Tsdown library          |
| `./module-runner`               | `./dist/vite/node/module-runner.js`      | Vite module runner      |
| `./rolldown`                    | `./dist/rolldown/index.mjs`              | Rolldown main entry     |
| `./rolldown/config`             | `./dist/rolldown/config.mjs`             | Rolldown config helpers |
| `./rolldown/experimental`       | `./dist/rolldown/experimental-index.mjs` | Experimental features   |
| `./rolldown/filter`             | `./dist/rolldown/filter-index.mjs`       | Filter utilities        |
| `./rolldown/parallelPlugin`     | `./dist/rolldown/parallel-plugin.mjs`    | Parallel plugin support |
| `./rolldown/parseAst`           | `./dist/rolldown/parse-ast-index.mjs`    | AST parsing             |
| `./rolldown/plugins`            | `./dist/rolldown/plugins-index.mjs`      | Built-in plugins        |
| `./rolldown/pluginutils`        | `./dist/pluginutils/index.js`            | Plugin utilities        |
| `./rolldown/pluginutils/filter` | `./dist/pluginutils/filter/index.js`     | Filter utilities        |
| `./types/*`                     | `./dist/vite/types/*`                    | Type definitions        |

---

## Source Directories

| Upstream Project        | Source Location                       | Relation       |
| ----------------------- | ------------------------------------- | -------------- |
| `@rolldown/pluginutils` | `../../rolldown/packages/pluginutils` | Git submodule  |
| `rolldown`              | `../../rolldown/packages/rolldown`    | Git submodule  |
| `vite`                  | `../../vite/packages/vite`            | Git submodule  |
| `tsdown`                | `node_modules/tsdown`                 | npm dependency |
| `vitepress`             | `node_modules/vitepress`              | npm dependency |

---

## Build Dependencies

| Package               | Purpose                               |
| --------------------- | ------------------------------------- |
| `rolldown`            | Bundler for building vite and tsdown  |
| `rolldown-plugin-dts` | TypeScript declaration bundling       |
| `@ast-grep/napi`      | Post-build module specifier rewriting |
| `oxc-parser`          | CJS require detection in tsdown       |
| `oxfmt`               | Code formatting for package.json      |
| `tinyglobby`          | File globbing for copying files       |

---

## Maintenance: Updating Bundled Versions

### Updating Vite

1. Update the `vite` git submodule to the new version
2. Run `pnpm -C packages/core build`
3. Verify `bundledVersions.vite` in `package.json` is updated
4. Test with `pnpm test`

### Updating Rolldown

1. Update the `rolldown` git submodule to the new version
2. Run `pnpm -C packages/core build`
3. Verify `bundledVersions.rolldown` in `package.json` is updated
4. Test with `pnpm test`

### Updating Tsdown

1. Update `tsdown` version in `devDependencies`
2. Run `pnpm install`
3. Run `pnpm -C packages/core build`
4. Check for new CJS dependencies (build will detect them automatically)
5. Verify `bundledVersions.tsdown` in `package.json` is updated
6. Test with `pnpm test`

### Updating Vitepress

1. Update `vitepress` version in `devDependencies`
2. Run `pnpm install`
3. Run `pnpm -C packages/core build`
4. Test documentation builds

---

## Build Commands

```bash
# Build the core package
pnpm -C packages/core build

# Release build (rewrites @rolldown/binding-* to vite-plus/binding)
RELEASE_BUILD=1 pnpm -C packages/core build
```

---

## Technical Reference

### Build Flow

```
1. bundleRolldownPluginutils()    Copy pre-built dist
2. bundleRolldown()               Copy + rewrite module specifiers
3. buildVite()                    Full Rolldown build with transforms
   ├── Apply RewriteImportsPlugin     Build-time import rewriting
   ├── Apply rewrite-static-paths     Fix VITE_PACKAGE_DIR constants
   ├── Run Rolldown build             Bundle vite source
   └── Copy and rewrite .d.ts files   Post-build specifier rewriting
4. bundleTsdown()                 Re-bundle with CJS handling
   ├── Bundle tsdown with Rolldown    Find CJS modules
   ├── buildCjsDeps()                 Bundle detected CJS deps
   └── Bundle types with dts plugin   Generate declarations
5. bundleVitepress()              Copy + rewrite vite imports
6. mergePackageJson()             Merge metadata + record versions
```

### Key Constants

```typescript
// Source directories
const rolldownPluginUtilsDir = resolve(
  projectDir,
  '..',
  '..',
  'rolldown',
  'packages',
  'pluginutils',
);
const rolldownSourceDir = resolve(projectDir, '..', '..', 'rolldown', 'packages', 'rolldown');
const rolldownViteSourceDir = resolve(projectDir, '..', '..', 'vite', 'packages', 'vite');
const tsdownSourceDir = resolve(projectDir, 'node_modules/tsdown');

// Package name used for rewrites
const targetPackage = '@voidzero-dev/vite-plus-core';
```

### Bundled Versions Tracking

The `bundledVersions` field in `package.json` records the exact versions of bundled upstream projects:

```json
{
  "bundledVersions": {
    "vite": "8.0.0-beta.8",
    "rolldown": "1.0.0-beta.60",
    "tsdown": "0.20.0-beta.4"
  }
}
```

This is automatically updated by `mergePackageJson()` during each build.

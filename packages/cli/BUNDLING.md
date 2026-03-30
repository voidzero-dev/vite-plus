# CLI Package Build Architecture

This document explains how `vite-plus` is built and how it re-exports from both the core and test packages to serve as a drop-in replacement for `vite`.

## Overview

The CLI package uses a **4-step build process**:

1. **TypeScript Compilation** - Compile TypeScript source to JavaScript
2. **NAPI Binding Build** - Compile Rust code to native Node.js bindings
3. **Core Package Export Sync** - Re-export `@voidzero-dev/vite-plus-core` under `./client`, `./types/*`, etc.
4. **Test Package Export Sync** - Re-export `@voidzero-dev/vite-plus-test` under `./test/*`

This architecture allows users to import everything from a single package (`vite-plus`) as a drop-in replacement for `vite`, without needing to know about the separate core and test packages.

## Build Steps

### Step 1: TypeScript Compilation (`buildCli`)

Compiles TypeScript source files using the TypeScript compiler API:

```typescript
const program = createProgram({
  rootNames: fileNames,
  options,
  host,
});
program.emit();
```

**Input**: `src/*.ts` files
**Output**: `dist/*.js`, `dist/*.d.ts`

### Step 2: NAPI Binding Build (`buildNapiBinding`)

Builds native Rust bindings using `@napi-rs/cli`:

```typescript
const cli = new NapiCli();
await cli.build({
  packageJsonPath: '../package.json',
  cwd: 'binding',
  platform: true,
  release: process.env.VP_CLI_DEBUG !== '1',
  esm: true,
});
```

**Input**: `binding/*.rs` (Rust source)
**Output**: `binding/*.node` (platform-specific binaries)

The build generates platform-specific native binaries and formats the generated JavaScript wrapper with `oxfmt`.

### Step 3: Core Package Export Sync (`syncCorePackageExports`)

Creates shim files that re-export from `@voidzero-dev/vite-plus-core`, enabling this package to be a drop-in replacement for upstream `vite`. This is critical for compatibility with existing Vite plugins and configurations.

**Prerequisites**: The core package must be built first (its `dist/vite/` directory must exist). See [Core Package Bundling](../core/BUNDLING.md) for details on how the core package bundles vite, rolldown, and tsdown.

**Export paths created**:

| Export Path          | Type       | Description                                                                             |
| -------------------- | ---------- | --------------------------------------------------------------------------------------- |
| `./client`           | Types only | Triple-slash reference for ambient type declarations (CSS modules, asset imports, etc.) |
| `./module-runner`    | JS + Types | Re-exports the Vite module runner for SSR/environments                                  |
| `./internal`         | JS + Types | Re-exports internal Vite APIs                                                           |
| `./dist/client/*`    | JS         | Client runtime files (`.mjs`, `.cjs`)                                                   |
| `./types/*`          | Types only | Type-only re-exports using `export type *`                                              |
| `./types/internal/*` | Blocked    | Set to `null` to prevent access to internal types                                       |

**Shim file examples**:

```typescript
// dist/client.d.ts (triple-slash reference for ambient types)
/// <reference types="@voidzero-dev/vite-plus-core/client" />

// dist/module-runner.js
export * from '@voidzero-dev/vite-plus-core/module-runner';

// dist/types/importMeta.d.ts (type-only export)
export type * from '@voidzero-dev/vite-plus-core/types/importMeta.d.ts';
```

**Note on export ordering**: In `package.json`, the `./types/internal/*` export (set to `null`) must appear before `./types/*` for correct precedence. More specific patterns must precede wildcards.

### Step 4: Test Package Export Sync (`syncTestPackageExports`)

Reads the test package's exports and creates shim files that re-export everything under `./test/*`:

```typescript
// For each test package export like "./browser-playwright"
// Creates a shim file: dist/test/browser-playwright.js
export * from '@voidzero-dev/vite-plus-test/browser-playwright';
```

**Input**: `../test/package.json` exports
**Output**: `dist/test/*.js`, `dist/test/*.d.ts`, updated `package.json` exports

---

## Output Structure

```
packages/cli/
├── dist/
│   ├── index.js              # Main entry (ESM)
│   ├── index.cjs             # Main entry (CJS)
│   ├── index.d.ts            # Type declarations
│   ├── bin.js                # CLI entry point
│   ├── client.d.ts           # ./client types (triple-slash ref)
│   ├── module-runner.js      # ./module-runner shim
│   ├── module-runner.d.ts
│   ├── internal.js           # ./internal shim
│   ├── internal.d.ts
│   ├── client/               # Synced client runtime files
│   │   ├── client.mjs        # ESM client shim
│   │   ├── client.d.ts
│   │   ├── env.mjs
│   │   └── ...
│   ├── types/                # Synced type definitions
│   │   ├── importMeta.d.ts   # Type shims (export type *)
│   │   ├── importGlob.d.ts
│   │   ├── customEvent.d.ts
│   │   └── ...
│   └── test/                 # Synced test exports
│       ├── index.js          # Re-exports @voidzero-dev/vite-plus-test
│       ├── index.cjs
│       ├── index.d.ts
│       ├── browser-playwright.js
│       ├── browser-playwright.d.ts
│       ├── plugins/
│       │   ├── runner.js
│       │   ├── utils.js
│       │   ├── spy.js
│       │   └── ... (33+ plugin shims)
│       └── ...
├── binding/
│   ├── index.js              # NAPI binding JS wrapper
│   ├── index.d.ts            # NAPI type declarations
│   └── *.node                # Platform-specific binaries
└── bin/
    └── vite                  # Shell entry point
```

---

## NAPI Targets

The CLI builds native bindings for the following platform targets:

| Target                      | Platform | Architecture | Output File                       |
| --------------------------- | -------- | ------------ | --------------------------------- |
| `aarch64-apple-darwin`      | macOS    | ARM64        | `vite-plus.darwin-arm64.node`     |
| `x86_64-apple-darwin`       | macOS    | x64          | `vite-plus.darwin-x64.node`       |
| `aarch64-unknown-linux-gnu` | Linux    | ARM64        | `vite-plus.linux-arm64-gnu.node`  |
| `x86_64-unknown-linux-gnu`  | Linux    | x64          | `vite-plus.linux-x64-gnu.node`    |
| `aarch64-pc-windows-msvc`   | Windows  | ARM64        | `vite-plus.win32-arm64-msvc.node` |
| `x86_64-pc-windows-msvc`    | Windows  | x64          | `vite-plus.win32-x64-msvc.node`   |

These targets are defined in `package.json` under the `napi.targets` field.

---

## Rolldown Native Binding Integration

The CLI package integrates with Rolldown at the native binding level, allowing vite-plus to ship as a self-contained package without requiring users to install separate `@rolldown/binding-*` packages.

### Conditional Compilation

Rolldown bindings are **optionally** compiled into the vite-plus native module via Cargo feature flags.

**In `binding/Cargo.toml`**:

```toml
[dependencies]
rolldown_binding = { workspace = true, optional = true }

[features]
rolldown = ["dep:rolldown_binding"]
```

**In `binding/src/lib.rs`**:

```rust
#[cfg(feature = "rolldown")]
pub extern crate rolldown_binding;
```

### Build-Time Feature Activation

The rolldown feature is only enabled during release builds:

```typescript
// In build.ts
await cli.build({
  features: process.env.RELEASE_BUILD ? ['rolldown'] : void 0,
  release: process.env.VP_CLI_DEBUG !== '1',
});
```

**When `RELEASE_BUILD=1`**:

1. Enables the `rolldown` Cargo feature
2. Compiles `rolldown_binding` into the `.node` file
3. Extracts `napi.dtsHeader` from rolldown's package.json for type definitions
4. Prepends custom type definitions to the generated `.d.ts` file

### Why Conditional Compilation?

| Build Type                  | rolldown Feature | Use Case                                |
| --------------------------- | ---------------- | --------------------------------------- |
| Development (`pnpm build`)  | Disabled         | Faster builds, smaller binaries         |
| Release (`RELEASE_BUILD=1`) | Enabled          | Full distribution with bundled rolldown |

### Module Specifier Rewriting

During release builds, the core package rewrites all `@rolldown/binding-*` imports to point to `vite-plus/binding`:

```typescript
// In packages/core/build.ts
if (process.env.RELEASE_BUILD) {
  // @rolldown/binding-darwin-arm64 → vite-plus/binding
  source = source.replace(/@rolldown\/binding-([a-z0-9-]+)/g, 'vite-plus/binding');
}
```

**Transformation examples**:

| Original Import                    | After Rewrite       |
| ---------------------------------- | ------------------- |
| `@rolldown/binding-darwin-arm64`   | `vite-plus/binding` |
| `@rolldown/binding-linux-x64-gnu`  | `vite-plus/binding` |
| `@rolldown/binding-win32-x64-msvc` | `vite-plus/binding` |

This means:

1. The bundled rolldown code in `@voidzero-dev/vite-plus-core/rolldown` resolves native bindings from `vite-plus/binding`
2. Users don't need to install separate `@rolldown/binding-*` platform packages
3. The single `.node` file contains both vite-plus task runner and rolldown bindings

### Native Binding Contents

When compiled with `RELEASE_BUILD=1`, the `.node` file contains:

| Component          | Source                             | Purpose                        |
| ------------------ | ---------------------------------- | ------------------------------ |
| `vite_task`        | `packages/cli/binding/src/lib.rs`  | Task runner session management |
| `rolldown_binding` | `rolldown/crates/rolldown_binding` | Rolldown bundler NAPI bindings |

### Export Chain

```
User imports 'vite-plus/rolldown'
  → packages/cli re-exports from @voidzero-dev/vite-plus-core/rolldown
    → packages/core/dist/rolldown/index.mjs
      → Native binding: vite-plus/binding (rewritten from @rolldown/binding-*)
        → binding/vite-plus.darwin-arm64.node (contains rolldown_binding)
```

### Platform-Specific Publishing

Native bindings are published as separate platform packages for optimal install size:

| Platform    | Published Package                         |
| ----------- | ----------------------------------------- |
| macOS ARM64 | `@voidzero-dev/vite-plus-darwin-arm64`    |
| macOS x64   | `@voidzero-dev/vite-plus-darwin-x64`      |
| Linux ARM64 | `@voidzero-dev/vite-plus-linux-arm64-gnu` |
| Linux x64   | `@voidzero-dev/vite-plus-linux-x64-gnu`   |
| Windows x64 | `@voidzero-dev/vite-plus-win32-x64-msvc`  |

These are automatically installed via `optionalDependencies` based on the user's platform.

See `publish-native-addons.ts` for the publishing pipeline.

---

## Core Package Export Sync Details

### Why Shim Files?

The CLI package creates thin shim files that re-export from `@voidzero-dev/vite-plus-core` rather than bundling the actual code. This approach:

1. **Enables drop-in replacement** - Users can replace `vite` with `vite-plus` without changing imports
2. **Keeps packages in sync** - No need to rebuild CLI when core package changes
3. **Reduces duplication** - No file copying, just re-exports
4. **Preserves module resolution** - Node.js resolves to the actual core package

**Note**: The `@voidzero-dev/vite-plus-core` package itself bundles multiple upstream projects (vite, rolldown, tsdown, vitepress). See [Core Package Bundling](../core/BUNDLING.md) for details.

### Export Mapping (Core)

| Upstream Vite Export | CLI Package Export        | Description                                |
| -------------------- | ------------------------- | ------------------------------------------ |
| `vite/client`        | `vite-plus/client`        | Ambient types for HMR, CSS modules, assets |
| `vite/module-runner` | `vite-plus/module-runner` | SSR/Environment module runner              |
| `vite/internal`      | `vite-plus/internal`      | Internal APIs                              |
| `vite/dist/client/*` | `vite-plus/dist/client/*` | Client runtime files                       |
| `vite/types/*`       | `vite-plus/types/*`       | Type definitions                           |

### Type-Only Exports

For `./types/*` exports, shim files use `export type *` syntax (TypeScript 5.0+) to ensure only type information is re-exported:

```typescript
// dist/types/importMeta.d.ts
export type * from '@voidzero-dev/vite-plus-core/types/importMeta.d.ts';
```

This is important because `./types/*` only exposes `.d.ts` files and should never include runtime code.

### Internal Types Blocking

The `./types/internal/*` export is set to `null` in package.json to block access to internal type definitions:

```json
"./types/internal/*": null,
"./types/*": { "types": "./dist/types/*" }
```

The `syncTypesDir()` helper skips the top-level `internal` directory when creating shims, since access is blocked at the exports level.

### Client Types (Triple-Slash Reference)

The `./client` export uses a triple-slash reference instead of a regular export because Vite's `client.d.ts` contains ambient type declarations (for CSS modules, assets, etc.) that should be globally available:

```typescript
// dist/client.d.ts
/// <reference types="@voidzero-dev/vite-plus-core/client" />
```

This allows TypeScript to pick up types like `import.meta.hot`, CSS module types, and asset imports without explicit imports.

---

## Test Package Export Sync Details

### Why Shim Files?

Instead of copying the actual dist files from the test package, we create thin shim files that re-export from `@voidzero-dev/vite-plus-test`. This approach:

1. **Keeps packages in sync** - No need to rebuild CLI when test package changes
2. **Reduces duplication** - No file copying, just re-exports
3. **Preserves module resolution** - Node.js resolves to the actual test package

### Export Mapping (Test)

All test package exports are mapped under `./test/*`:

| Test Package Export                               | CLI Package Export                  |
| ------------------------------------------------- | ----------------------------------- |
| `@voidzero-dev/vite-plus-test`                    | `vite-plus/test`                    |
| `@voidzero-dev/vite-plus-test/browser`            | `vite-plus/test/browser`            |
| `@voidzero-dev/vite-plus-test/browser-playwright` | `vite-plus/test/browser-playwright` |
| `@voidzero-dev/vite-plus-test/plugins/runner`     | `vite-plus/test/plugins/runner`     |

### Conditional Export Handling

The sync handles complex conditional exports with `import`/`require`/`node`/`types` conditions.

**Test package's main export** (`"."`):

```json
".": {
  "import": { "types": "...", "node": "...", "default": "..." },
  "require": { "types": "...", "default": "..." }
}
```

**Becomes CLI package export** (`"./test"`):

```json
"./test": {
  "import": {
    "types": "./dist/test/index.d.ts",
    "node": "./dist/test/index.js",
    "default": "./dist/test/index.js"
  },
  "require": {
    "types": "./dist/test/index.d.cts",
    "default": "./dist/test/index.cjs"
  }
}
```

For each condition, appropriate shim files are created:

- `.js` for ESM imports
- `.cjs` for CommonJS requires
- `.d.ts` / `.d.cts` for type declarations

### Shim File Contents

**ESM shim** (`dist/test/browser-playwright.js`):

```javascript
export * from '@voidzero-dev/vite-plus-test/browser-playwright';
```

**CJS shim** (`dist/test/index.cjs`):

```javascript
module.exports = require('@voidzero-dev/vite-plus-test');
```

**Type shim** (`dist/test/browser-playwright.d.ts`):

```typescript
import '@voidzero-dev/vite-plus-test/browser-playwright';
export * from '@voidzero-dev/vite-plus-test/browser-playwright';
```

Note: Type shims include a side-effect import to preserve module augmentations (e.g., `toMatchSnapshot` on the `Assertion` interface).

---

## Build Dependencies

| Package        | Purpose                          |
| -------------- | -------------------------------- |
| `@napi-rs/cli` | NAPI build toolchain for Rust    |
| `oxfmt`        | Code formatting for generated JS |
| `typescript`   | TypeScript compilation           |

---

## Debug Mode

To build with debug (unoptimized) Rust bindings:

```bash
VP_CLI_DEBUG=1 pnpm build
```

This sets `release: false` in the NAPI build options, producing larger but faster-to-compile debug binaries.

---

## Build Commands

```bash
# Build the CLI package (requires core package to be built first)
pnpm -C packages/cli build

# Build from monorepo root (builds all dependencies first)
pnpm build --filter vite-plus

# Debug build
VP_CLI_DEBUG=1 pnpm -C packages/cli build
```

---

## Package Exports

After building, the CLI package exports:

| Export Path                 | Description                         |
| --------------------------- | ----------------------------------- |
| `.`                         | Main entry (CLI utilities)          |
| `./client`                  | Client types (ambient declarations) |
| `./module-runner`           | Vite module runner for SSR          |
| `./internal`                | Internal Vite APIs                  |
| `./dist/client/*`           | Client runtime files                |
| `./types/*`                 | Type definitions                    |
| `./bin`                     | CLI binary entry point              |
| `./binding`                 | NAPI native binding                 |
| `./test`                    | Test package main entry             |
| `./test/browser`            | Browser testing utilities           |
| `./test/browser-playwright` | Playwright integration              |
| `./test/plugins/*`          | Plugin shims for pnpm overrides     |
| `./package.json`            | Package metadata                    |

See `package.json` for the complete list of exports.

---

## Technical Reference

### Build Flow

```
1. buildCli()                TypeScript compilation -> dist/*.js
2. buildNapiBinding()        Rust -> binding/*.node (per platform)
3. syncCorePackageExports()  Read core pkg dist -> dist/client/, dist/types/
   ├── createClientShim()        Triple-slash reference for ./client
   ├── createModuleRunnerShim()  JS + types for ./module-runner
   ├── createInternalShim()      JS + types for ./internal
   ├── syncClientDir()           Shims for ./dist/client/*
   └── syncTypesDir()            Type-only shims for ./types/*
4. syncTestPackageExports()  Read test pkg exports -> dist/test/*
   ├── createShimForExport()     Generate shim files
   ├── createConditionalShim()   Handle import/require conditions
   └── updateCliPackageJson()    Update exports in package.json
```

### Key Constants

```typescript
// Core package name for Vite compatibility exports
const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';

// Test package name for re-exports
const TEST_PACKAGE_NAME = '@voidzero-dev/vite-plus-test';
```

### Package.json Exports Management

The `exports` field in `package.json` has two categories: **manual** and **automated**.

#### Manual exports

All non-`./test*` exports are manually maintained in `package.json`. These fall into two groups:

**CLI-native exports** — point to CLI's own compiled TypeScript (built by `buildCli()` via tsc):

| Export           | Description                |
| ---------------- | -------------------------- |
| `.`              | Main entry (CLI utilities) |
| `./bin`          | CLI binary entry point     |
| `./binding`      | NAPI native binding        |
| `./lint`         | Lint utilities             |
| `./pack`         | Pack utilities             |
| `./package.json` | Package metadata           |

**Core shim exports** — point to shim files auto-generated by `syncCorePackageExports()` that re-export from `@voidzero-dev/vite-plus-core`. The shim files are regenerated on each build, but the `package.json` entries themselves are manual:

| Export               | Description                                                             |
| -------------------- | ----------------------------------------------------------------------- |
| `./client`           | Triple-slash reference for ambient type declarations (CSS modules, etc) |
| `./module-runner`    | Vite module runner for SSR/environments                                 |
| `./internal`         | Internal Vite APIs                                                      |
| `./dist/client/*`    | Client runtime files                                                    |
| `./types/internal/*` | Blocked (`null`) to prevent access to internal types                    |
| `./types/*`          | Type-only re-exports                                                    |

**Note**: The core package's own exports (which the shims point to) are generated upstream by `packages/tools/src/sync-remote-deps.ts`. See [Core Package Bundling](../core/BUNDLING.md) for details.

#### Automated exports (`./test/*`)

All `./test*` exports are fully managed by `syncTestPackageExports()`. The build script:

1. Reads `packages/test/package.json` exports
2. Creates shim files in `dist/test/`
3. Removes old `./test*` exports from `package.json`
4. Merges in newly generated test exports
5. Ensures `dist/test` is in the `files` array

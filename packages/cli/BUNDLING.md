# CLI Package Build Architecture

This document explains how `vite-plus` is built and how it re-exports from `@voidzero-dev/vite-plus-core` (bundled vite/rolldown/tsdown) and from upstream `vitest` to serve as a drop-in replacement for `vite`.

## Overview

The CLI package uses a **4-step build process**:

1. **tsdown Build** - Bundle all CLI entry points via tsdown
2. **NAPI Binding Build** - Compile Rust code to native Node.js bindings
3. **Core Package Export Sync** - Re-export `@voidzero-dev/vite-plus-core` under `./client`, `./types/*`, etc.
4. **Test Package Export Sync** - Re-export upstream `vitest` under `./test/*`

This architecture allows users to import everything from a single package (`vite-plus`) as a drop-in replacement for `vite`, without needing to know about the separate `@voidzero-dev/vite-plus-core` bundle or `vitest`.

## Build Steps

### Step 1: tsdown Build (`buildWithTsdown`)

Bundles all CLI entry points using tsdown (configured in `tsdown.config.ts`). The config defines two builds:

**ESM build** — bundles all entry points to `dist/`:

- Public API entries: `bin`, `index`, `define-config`, `fmt`, `lint`, `pack`, `pack-bin`
- Global command entries: `create`, `migrate`, `version`, `config`, `mcp`, `staged`
- All third-party dependencies are inlined at build time
- Only packages that must be resolved at runtime stay external (NAPI binding, `@voidzero-dev/vite-plus-core`, `vitest`, `oxfmt`, `oxlint`)
- Code splitting creates shared chunks for code used by multiple entries
- DTS (`.d.ts`) files are generated for all entries

**CJS build** — produces dual-format output for:

- `define-config.ts` → `dist/define-config.cjs`
- `index.cts` → `dist/index.cjs`

**Input**: `src/**/*.ts`, `src/**/*.cts`
**Output**: `dist/*.js`, `dist/*.cjs`, `dist/*.d.ts`, `dist/*-<hash>.js` (shared chunks)

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

Reads vitest's exports plus the three `@vitest/browser-*` provider packages and creates shim files that re-export everything under `./test/*`:

```typescript
// For each vitest export like "./node"
// Creates a shim file: dist/test/node.js
export * from 'vitest/node';

// For each @vitest/browser-* provider, two shim surfaces are projected:
//   dist/test/browser-playwright.js          (matches old wrapper path)
//   dist/test/browser/providers/playwright.js (alias path)
export * from '@vitest/browser-playwright';
```

Provider `.d.ts` shims are NOT bare re-exports — see the [Provider Type Identity](#why-provider-dts-shims-are-inlined) note below.

**Input**: resolved `vitest/package.json` exports plus each `@vitest/browser-*` package's exports (all resolved via `createRequire`)
**Output**: `dist/test/*.js`, `dist/test/*.d.ts`, updated `package.json` exports

---

## Output Structure

```
packages/cli/
├── dist/
│   ├── bin.js                # CLI entry point (bundled)
│   ├── index.js              # Main entry (ESM, bundled)
│   ├── index.cjs             # Main entry (CJS)
│   ├── index.d.ts            # Type declarations
│   ├── define-config.js      # Config helper (ESM)
│   ├── define-config.cjs     # Config helper (CJS)
│   ├── define-config.d.ts
│   ├── fmt.js                # Re-exports oxfmt
│   ├── lint.js               # Re-exports oxlint types
│   ├── pack.js               # Re-exports vite-plus-core/pack
│   ├── pack-bin.js           # tsdown CLI for `vp pack`
│   ├── create.js             # Global command: vp create
│   ├── migrate.js            # Global command: vp migrate
│   ├── version.js            # Global command: vp --version
│   ├── config.js             # Global command: vp config
│   ├── mcp.js                # Global command: vp mcp
│   ├── staged.js             # Global command: vp staged
│   ├── *-<hash>.js           # Shared chunks (code splitting)
│   ├── versions.js           # Generated tool versions
│   ├── client.d.ts           # ./client types (triple-slash ref)
│   ├── module-runner.js      # ./module-runner shim
│   ├── internal.js           # ./internal shim
│   ├── client/               # Synced client runtime files
│   ├── types/                # Synced type definitions
│   └── test/                 # Synced test exports
├── binding/
│   ├── index.js              # NAPI binding JS wrapper
│   ├── index.d.ts            # NAPI type declarations
│   └── *.node                # Platform-specific binaries
└── bin/
    └── vp                    # Shell entry point
```

---

## NAPI Targets

The CLI builds native bindings for the following platform targets:

| Target                       | Platform | Architecture | Output File                       |
| ---------------------------- | -------- | ------------ | --------------------------------- |
| `aarch64-apple-darwin`       | macOS    | ARM64        | `vite-plus.darwin-arm64.node`     |
| `x86_64-apple-darwin`        | macOS    | x64          | `vite-plus.darwin-x64.node`       |
| `aarch64-unknown-linux-gnu`  | Linux    | ARM64        | `vite-plus.linux-arm64-gnu.node`  |
| `aarch64-unknown-linux-musl` | Linux    | ARM64        | `vite-plus.linux-arm64-musl.node` |
| `x86_64-unknown-linux-gnu`   | Linux    | x64          | `vite-plus.linux-x64-gnu.node`    |
| `x86_64-unknown-linux-musl`  | Linux    | x64          | `vite-plus.linux-x64-musl.node`   |
| `aarch64-pc-windows-msvc`    | Windows  | ARM64        | `vite-plus.win32-arm64-msvc.node` |
| `x86_64-pc-windows-msvc`     | Windows  | x64          | `vite-plus.win32-x64-msvc.node`   |

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

During release builds, the core package rewrites each `@rolldown/binding-*` import to the matching Vite+ platform package:

```typescript
// In packages/core/build.ts
if (process.env.RELEASE_BUILD) {
  source = source.replace(/@rolldown\/binding-([a-z0-9-]+)/g, '@voidzero-dev/vite-plus-$1');
}
```

**Transformation examples**:

| Original Import                    | After Rewrite                            |
| ---------------------------------- | ---------------------------------------- |
| `@rolldown/binding-darwin-arm64`   | `@voidzero-dev/vite-plus-darwin-arm64`   |
| `@rolldown/binding-linux-x64-gnu`  | `@voidzero-dev/vite-plus-linux-x64-gnu`  |
| `@rolldown/binding-win32-x64-msvc` | `@voidzero-dev/vite-plus-win32-x64-msvc` |

This means:

1. The bundled rolldown code in `@voidzero-dev/vite-plus-core/rolldown` resolves native bindings from core's declared optional platform dependencies
2. Users don't need to install separate `@rolldown/binding-*` platform packages
3. The platform `.node` file contains both vite-plus task runner and rolldown bindings
4. The `vite-plus/binding` export remains available as a compatibility entrypoint for direct consumers

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
      → Native binding: @voidzero-dev/vite-plus-darwin-arm64
        → vite-plus.darwin-arm64.node (contains rolldown_binding)
```

`vite-plus/binding` still exists for compatibility and uses the same platform packages under the CLI package's public export.

### Platform-Specific Publishing

Native bindings are published as separate platform packages for optimal install size:

| Platform          | Published Package                          |
| ----------------- | ------------------------------------------ |
| macOS ARM64       | `@voidzero-dev/vite-plus-darwin-arm64`     |
| macOS x64         | `@voidzero-dev/vite-plus-darwin-x64`       |
| Linux ARM64 glibc | `@voidzero-dev/vite-plus-linux-arm64-gnu`  |
| Linux ARM64 musl  | `@voidzero-dev/vite-plus-linux-arm64-musl` |
| Linux x64 glibc   | `@voidzero-dev/vite-plus-linux-x64-gnu`    |
| Linux x64 musl    | `@voidzero-dev/vite-plus-linux-x64-musl`   |
| Windows ARM64     | `@voidzero-dev/vite-plus-win32-arm64-msvc` |
| Windows x64       | `@voidzero-dev/vite-plus-win32-x64-msvc`   |

These are automatically installed via `optionalDependencies` based on the user's platform. The CLI and core packages both declare the platform packages they may load.

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

Instead of copying vitest's dist files, we create thin shim files that re-export from `vitest`. This approach:

1. **Keeps packages in sync** - No need to rebuild CLI when vitest is upgraded
2. **Reduces duplication** - No file copying, just re-exports
3. **Preserves module resolution** - Node.js resolves to the actual installed vitest

### Export Mapping (Test)

Every entry under vitest's own `exports` is shimmed under `./test/*` (wildcard exports and `./package.json` are skipped). The shim is purely a re-export — `vite-plus/test` and friends are aliases for the matching subpath of upstream `vitest`. Examples:

| Vitest Export      | CLI Package Export         |
| ------------------ | -------------------------- |
| `vitest`           | `vite-plus/test`           |
| `vitest/browser`   | `vite-plus/test/browser`   |
| `vitest/node`      | `vite-plus/test/node`      |
| `vitest/config`    | `vite-plus/test/config`    |
| `vitest/reporters` | `vite-plus/test/reporters` |

The full set is regenerated on every build from the upstream vitest `package.json`, so the exact list tracks vitest itself.

In addition to vitest's own exports, the three `@vitest/browser-*` provider packages are projected under two parallel surfaces so existing user code keeps resolving after the deleted `@voidzero-dev/vite-plus-test` wrapper:

| Provider Package              | CLI Package Exports                                                                  |
| ----------------------------- | ------------------------------------------------------------------------------------ |
| `@vitest/browser-playwright`  | `vite-plus/test/browser-playwright`, `vite-plus/test/browser/providers/playwright`   |
| `@vitest/browser-preview`     | `vite-plus/test/browser-preview`, `vite-plus/test/browser/providers/preview`         |
| `@vitest/browser-webdriverio` | `vite-plus/test/browser-webdriverio`, `vite-plus/test/browser/providers/webdriverio` |

Each provider's own subpaths (e.g. `./context`) are mirrored under both alias prefixes.

> **Note — webdriverio and playwright are opt-in.** `@vitest/browser` (base) and `@vitest/browser-preview` stay bundled **runtime dependencies** of `vite-plus` (and are stripped from users' manifests during migration) because neither carries a heavy non-optional peer. `@vitest/browser-webdriverio` and `@vitest/browser-playwright` are now vite-plus **devDependencies + optional peerDependencies** — each is kept as a devDependency so build-time shim generation can still emit the `./test/browser-webdriverio*` / `./test/browser-playwright*` exports (the export/shim surfaces above are unchanged), but neither is a bundled runtime dep. They are optional peers because each drags a non-optional framework peer (`webdriverio` / `playwright`) that non-browser consumers must not be forced to install. Users targeting a provider instead **keep** it in their **own** dependencies via `vp migrate` (pinned to the bundled vitest version, with its framework peer ensured), so their rewritten `vite-plus/test/browser-webdriverio` / `vite-plus/test/browser-playwright` imports resolve.

#### Why provider d.ts shims are inlined

Provider `.d.ts` shims are NOT plain `export * from '@vitest/browser-playwright'` re-exports — they inline the upstream `.d.ts` content with `vitest/node` / `vitest/browser` / `@vitest/browser*` bare specifiers rewritten to relative paths inside `dist/test/`. The two private shims `dist/test/_at-vitest-browser.d.ts` and `dist/test/_at-vitest-browser/context.d.ts` re-export `@vitest/browser`/`@vitest/browser/context` and are referenced from those rewrites.

This avoids a pnpm-edge type-identity split: when the upstream `.d.ts` is loaded by reference (`export * from '@vitest/browser-playwright'`), TypeScript resolves its internal `import { BrowserProvider } from 'vitest/node'` through the provider package's own pnpm-edge, which can be a different vitest copy than the one a user's `vite.config.ts` sees through `vite-plus`. The mismatch produces two structurally identical but nominally distinct `BrowserProvider` types, so `provider: playwright()` fails the user's typecheck. Rewriting the specifiers routes every type import through vite-plus's own subpath shims, guaranteeing a single vitest identity across the user's whole config.

### Conditional Export Handling

The sync handles complex conditional exports with `import`/`require`/`node`/`types` conditions.

**Vitest's main export** (`"."`):

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

**ESM shim** (`dist/test/browser.js`):

```javascript
export * from 'vitest/browser';
```

**CJS shim** (`dist/test/index.cjs`):

```javascript
module.exports = require('vitest');
```

**Type shim** (`dist/test/browser.d.ts`):

```typescript
import 'vitest/browser';
export * from 'vitest/browser';
```

Note: Type shims include a side-effect import to preserve module augmentations (e.g., `toMatchSnapshot` on the `Assertion` interface).

---

## Build Dependencies

| Package        | Purpose                          |
| -------------- | -------------------------------- |
| `@napi-rs/cli` | NAPI build toolchain for Rust    |
| `oxfmt`        | Code formatting for generated JS |
| `tsdown`       | TypeScript bundling              |

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
1. buildWithTsdown()         tsdown bundle -> dist/*.js, dist/*.d.ts
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

// Test package name for re-exports (vitest itself, not a bundled wrapper)
const TEST_PACKAGE_NAME = 'vitest';
```

### Package.json Exports Management

The `exports` field in `package.json` has two categories: **manual** and **automated**.

#### Manual exports

All non-`./test*` exports are manually maintained in `package.json`. These fall into two groups:

**CLI-native exports** — point to CLI's own bundled TypeScript (built by `buildWithTsdown()` via tsdown):

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

1. Reads vitest's `package.json` exports (resolved via `createRequire`)
2. Creates shim files in `dist/test/`
3. Removes old `./test*` exports from `package.json`
4. Merges in newly generated test exports
5. Ensures `dist/test` is in the `files` array

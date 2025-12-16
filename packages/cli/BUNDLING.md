# CLI Package Build Architecture

This document explains how `@voidzero-dev/vite-plus` is built and how it re-exports the test package.

## Overview

The CLI package uses a **3-step build process**:

1. **TypeScript Compilation** - Compile TypeScript source to JavaScript
2. **NAPI Binding Build** - Compile Rust code to native Node.js bindings
3. **Test Package Export Sync** - Re-export `@voidzero-dev/vite-plus-test` under `./test/*`

This allows users to import everything from a single package (`@voidzero-dev/vite-plus`) instead of needing to know about the separate test package.

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
  release: process.env.VITE_PLUS_CLI_DEBUG !== '1',
  esm: true,
});
```

**Input**: `binding/*.rs` (Rust source)
**Output**: `binding/*.node` (platform-specific binaries)

The build generates platform-specific native binaries and formats the generated JavaScript wrapper with `oxfmt`.

### Step 3: Test Package Export Sync (`syncTestPackageExports`)

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

## Test Package Export Sync Details

### Why Shim Files?

Instead of copying the actual dist files from the test package, we create thin shim files that re-export from `@voidzero-dev/vite-plus-test`. This approach:

1. **Keeps packages in sync** - No need to rebuild CLI when test package changes
2. **Reduces duplication** - No file copying, just re-exports
3. **Preserves module resolution** - Node.js resolves to the actual test package

### Export Mapping

All test package exports are mapped under `./test/*`:

| Test Package Export                               | CLI Package Export                                |
| ------------------------------------------------- | ------------------------------------------------- |
| `@voidzero-dev/vite-plus-test`                    | `@voidzero-dev/vite-plus/test`                    |
| `@voidzero-dev/vite-plus-test/browser`            | `@voidzero-dev/vite-plus/test/browser`            |
| `@voidzero-dev/vite-plus-test/browser-playwright` | `@voidzero-dev/vite-plus/test/browser-playwright` |
| `@voidzero-dev/vite-plus-test/plugins/runner`     | `@voidzero-dev/vite-plus/test/plugins/runner`     |

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
export * from '@voidzero-dev/vite-plus-test/browser-playwright';
```

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
VITE_PLUS_CLI_DEBUG=1 pnpm build
```

This sets `release: false` in the NAPI build options, producing larger but faster-to-compile debug binaries.

---

## Build Commands

```bash
# Build the CLI package
pnpm -C packages/cli build

# Build from monorepo root
pnpm build --filter @voidzero-dev/vite-plus

# Debug build
VITE_PLUS_CLI_DEBUG=1 pnpm -C packages/cli build
```

---

## Package Exports

After building, the CLI package exports:

| Export Path                 | Description                     |
| --------------------------- | ------------------------------- |
| `.`                         | Main entry (CLI utilities)      |
| `./bin`                     | CLI binary entry point          |
| `./binding`                 | NAPI native binding             |
| `./test`                    | Test package main entry         |
| `./test/browser`            | Browser testing utilities       |
| `./test/browser-playwright` | Playwright integration          |
| `./test/plugins/*`          | Plugin shims for pnpm overrides |
| `./package.json`            | Package metadata                |

See `package.json` for the complete list of exports.

---

## Technical Reference

### Build Flow

```
1. buildCli()              TypeScript compilation -> dist/*.js
2. buildNapiBinding()      Rust -> binding/*.node (per platform)
3. syncTestPackageExports() Read test pkg exports -> dist/test/*
   ├── createShimForExport()    Generate shim files
   ├── createConditionalShim()  Handle import/require conditions
   └── updateCliPackageJson()   Update exports in package.json
```

### Key Constants

```typescript
// Test package name for re-exports
const TEST_PACKAGE_NAME = '@voidzero-dev/vite-plus-test';
```

### Package.json Auto-Update

The build script automatically updates `package.json`:

1. Removes old `./test/*` exports
2. Adds new exports from test package
3. Ensures `dist/test` is in the `files` array

This keeps the CLI package exports in sync with the test package without manual maintenance.

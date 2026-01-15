# Test Package Bundling Architecture

This document explains how `@voidzero-dev/vite-plus-test` bundles vitest and its dependencies.

## Overview

The test package uses a **hybrid bundling strategy**:

1. **COPY** all `@vitest/*` packages (preserves browser/Node.js separation)
2. **BUNDLE** only leaf dependencies like `chai`, `pathe` (reduces install size)
3. **Separate entries** (`index.js` vs `index-node.js`) prevent Node.js code from loading in browsers

This approach avoids the critical issue of Rolldown creating shared chunks that mix Node.js-only code (like `__vite__injectQuery`) with browser code, which causes runtime crashes.

## Dependencies Classification

### Copied Packages (`dist/@vitest/`)

These 11 `@vitest/*` packages are **copied** (not bundled) to preserve their original file structure:

| Package                       | Purpose                                              |
| ----------------------------- | ---------------------------------------------------- |
| `@vitest/runner`              | Test runner core                                     |
| `@vitest/utils`               | Utilities (source-map, error, display, timers, etc.) |
| `@vitest/spy`                 | Spy/mock implementation                              |
| `@vitest/expect`              | Assertion library                                    |
| `@vitest/snapshot`            | Snapshot testing                                     |
| `@vitest/mocker`              | Module mocking (node, browser, automock)             |
| `@vitest/pretty-format`       | Output formatting                                    |
| `@vitest/browser`             | Browser testing support                              |
| `@vitest/browser-playwright`  | Playwright integration                               |
| `@vitest/browser-webdriverio` | WebdriverIO integration                              |
| `@vitest/browser-preview`     | Preview (testing-library) integration                |

**Why copy instead of bundle?** Bundling would create shared chunks that mix browser-safe and Node.js-only code. Copying preserves the original separation.

### Bundled Leaf Dependencies (`dist/vendor/`)

These packages are bundled using Rolldown into `dist/vendor/*.mjs`:

| Package               | Purpose                             |
| --------------------- | ----------------------------------- |
| `chai`                | Assertion library (core of expect)  |
| `pathe`               | Path utilities                      |
| `tinyrainbow`         | Terminal colors                     |
| `magic-string`        | String manipulation for source maps |
| `estree-walker`       | AST traversal                       |
| `why-is-node-running` | Debug tool for hanging processes    |

These were moved from `dependencies` to `devDependencies` since they're bundled.

### Runtime Dependencies (NOT Bundled)

These remain in `dependencies` and are installed with the package:

| Package           | Reason Not Bundled                            |
| ----------------- | --------------------------------------------- |
| `sirv`            | Static file server - complex runtime behavior |
| `ws`              | WebSocket server - native bindings            |
| `pixelmatch`      | Image comparison - optional feature           |
| `pngjs`           | PNG handling - optional feature               |
| `es-module-lexer` | ESM parsing - small, used at runtime          |
| `expect-type`     | Type testing - small                          |
| `obug`            | Debugging - small                             |
| `picomatch`       | Glob matching - small                         |
| `std-env`         | Environment detection - small                 |
| `tinybench`       | Benchmarking - optional feature               |
| `tinyexec`        | Command execution - small                     |
| `tinyglobby`      | File globbing - small                         |

### External Blocklist (Must NOT Bundle)

These packages are explicitly kept external in `EXTERNAL_BLOCKLIST` during the Rolldown build:

| Package            | Reason                                    |
| ------------------ | ----------------------------------------- |
| `playwright`       | Native bindings, user must install        |
| `webdriverio`      | Native bindings, user must install        |
| `debug`            | Environment detection breaks when bundled |
| `happy-dom`        | Optional peer dependency                  |
| `jsdom`            | Optional peer dependency                  |
| `@edge-runtime/vm` | Optional peer dependency                  |
| `msw`, `msw/*`     | Optional peer dependency for mocking      |

### Browser Plugin Exclude List

Additionally, these packages are added to the **browser plugin's exclude list** (in `patchVitestBrowserPackage`), which prevents Vite's optimizer from bundling them during browser tests:

| Package               | Reason                                           |
| --------------------- | ------------------------------------------------ |
| `lightningcss`        | Native bindings                                  |
| `@tailwindcss/oxide`  | Native bindings                                  |
| `tailwindcss`         | Pulls in @tailwindcss/oxide                      |
| `@vitest/browser`     | Needs vendor-aliases plugin resolution           |
| `@vitest/ui`          | Optional peer dependency                         |
| `@vitest/mocker/node` | Imports @voidzero-dev/vite-plus-core (Node-only) |

This is a different mechanism than `EXTERNAL_BLOCKLIST` - it controls runtime optimization, not build-time bundling.

---

## Migration Guide

For maintainers developing the vitest/vite migration feature, here are the transformations needed.

### Import Rewrites

| Original Import                      | Rewritten Import                                          |
| ------------------------------------ | --------------------------------------------------------- |
| `from "@vitest/browser-playwright"`  | `from "@voidzero-dev/vite-plus-test/browser-playwright"`  |
| `from "@vitest/browser-webdriverio"` | `from "@voidzero-dev/vite-plus-test/browser-webdriverio"` |
| `from "@vitest/browser-preview"`     | `from "@voidzero-dev/vite-plus-test/browser-preview"`     |
| `from "vite"`                        | `from "@voidzero-dev/vite-plus-core"`                     |
| `from "vite/module-runner"`          | `from "@voidzero-dev/vite-plus-core/module-runner"`       |

**Note:** When using pnpm overrides, you have three options for browser provider imports:

- `vitest/browser-playwright` (or `vitest/browser-webdriverio`, `vitest/browser-preview`) - works when `vitest` is overridden to our package (Recommended)
- `@voidzero-dev/vite-plus-test/browser-playwright` - direct import from test package
- `vite-plus/test/plugins/browser-playwright` - direct import from CLI package

Importing from `@vitest/browser-*` packages directly requires additional overrides for those specific packages.

### package.json Changes

**Remove these devDependencies** (now bundled):

```json
{
  "devDependencies": {
    "@vitest/browser": "...", // Remove
    "@vitest/browser-playwright": "...", // Remove (if using playwright)
    "@vitest/browser-webdriverio": "...", // Remove (if using webdriverio)
    "@vitest/browser-preview": "...", // Remove (if using testing-library)
    "@vitest/ui": "..." // Remove (peer dep, not bundled but optional)
  }
}
```

**Add pnpm overrides**:

```yaml
# pnpm-workspace.yaml
overrides:
  vite: 'file:path/to/vite-plus-core.tgz'
  vitest: 'file:path/to/vite-plus-test.tgz'
  '@vitest/browser': 'file:path/to/vite-plus-test.tgz'
  '@vitest/browser-playwright': 'file:path/to/vite-plus-test.tgz'
  '@vitest/browser-webdriverio': 'file:path/to/vite-plus-test.tgz'
  '@vitest/browser-preview': 'file:path/to/vite-plus-test.tgz'
```

Or using npm package names:

```yaml
overrides:
  vite: 'npm:@voidzero-dev/vite-plus-core'
  vitest: 'npm:@voidzero-dev/vite-plus-test'
  '@vitest/browser': 'npm:@voidzero-dev/vite-plus-test'
  '@vitest/browser-playwright': 'npm:@voidzero-dev/vite-plus-test'
  '@vitest/browser-webdriverio': 'npm:@voidzero-dev/vite-plus-test'
  '@vitest/browser-preview': 'npm:@voidzero-dev/vite-plus-test'
```

### Config File Updates

```typescript
// Before (playwright)
import { playwright } from '@vitest/browser-playwright';

// After - Option 1 (Recommended): Via vitest subpath (works when vitest is overridden)
import { playwright } from 'vitest/browser-playwright';

// After - Option 2: Direct import from test package
import { playwright } from '@voidzero-dev/vite-plus-test/browser-playwright';

// After - Option 3: Direct import from CLI package
import { playwright } from 'vite-plus/test/plugins/browser-playwright';
```

Similarly for WebdriverIO:

```typescript
import { webdriverio } from 'vitest/browser-webdriverio';
```

And for Preview (testing-library):

```typescript
import { preview } from 'vitest/browser-preview';
```

### Plugin Exports for pnpm Overrides

The package provides `./plugins/*` exports to enable pnpm overrides for all `@vitest/*` packages:

```
@vitest/runner              -> @voidzero-dev/vite-plus-test/plugins/runner
@vitest/utils               -> @voidzero-dev/vite-plus-test/plugins/utils
@vitest/utils/error         -> @voidzero-dev/vite-plus-test/plugins/utils-error
@vitest/spy                 -> @voidzero-dev/vite-plus-test/plugins/spy
@vitest/expect              -> @voidzero-dev/vite-plus-test/plugins/expect
@vitest/snapshot            -> @voidzero-dev/vite-plus-test/plugins/snapshot
@vitest/mocker              -> @voidzero-dev/vite-plus-test/plugins/mocker
@vitest/pretty-format       -> @voidzero-dev/vite-plus-test/plugins/pretty-format
@vitest/browser             -> @voidzero-dev/vite-plus-test/plugins/browser
@vitest/browser-playwright  -> @voidzero-dev/vite-plus-test/plugins/browser-playwright
@vitest/browser-webdriverio -> @voidzero-dev/vite-plus-test/plugins/browser-webdriverio
@vitest/browser-preview     -> @voidzero-dev/vite-plus-test/plugins/browser-preview
```

---

## Ensuring Bundle Stability

### Build-time Validation

The build script includes `validateExternalDeps()` which:

1. Scans all bundled JS files using `oxc-parser`
2. Extracts all external import specifiers
3. Verifies every external dependency is declared in `dependencies` or `peerDependencies`
4. Reports undeclared externals that would fail at runtime

If this validation fails, the build will report which packages need to be added.

### Testing Strategy

| Test Type                                                       | What It Validates                                  |
| --------------------------------------------------------------- | -------------------------------------------------- |
| **Snap tests** (`packages/cli/snap-tests/vitest-browser-mode/`) | Browser mode works after bundling                  |
| **Ecosystem CI** (`ecosystem-ci/`)                              | Real-world projects work with bundled vitest       |
| **CI workflows**                                                | Multi-platform validation (Ubuntu, Windows, macOS) |

### Vitest Upgrade Checklist

When upgrading the vitest version:

1. **Update version** in `packages/test/package.json`:
   ```json
   {
     "devDependencies": {
       "vitest-dev": "^NEW_VERSION",
       "@vitest/runner": "NEW_VERSION",
       "@vitest/utils": "NEW_VERSION"
       // ... all @vitest/* packages
     }
   }
   ```

2. **Run build**:
   ```bash
   pnpm -C packages/test build
   ```

3. **Check for new externals**: If `validateExternalDeps()` reports new undeclared dependencies:
   - Add to `dependencies` if it should be installed at runtime
   - Add to `EXTERNAL_BLOCKLIST` if it should remain external (native bindings, optional)
   - If it's a new leaf dep, it will be automatically bundled

4. **Run tests**:
   ```bash
   pnpm test
   ```

5. **Run ecosystem CI**:
   ```bash
   pnpm -C ecosystem-ci test
   ```

### Common Upgrade Issues

| Issue                   | Cause                          | Solution                                           |
| ----------------------- | ------------------------------ | -------------------------------------------------- |
| New undeclared external | New vitest dependency          | Add to `dependencies` or `EXTERNAL_BLOCKLIST`      |
| Browser test crashes    | Node.js code leaked to browser | Check import rewriting in `rewriteVitestImports()` |
| Missing export          | New @vitest/* subpath export   | Add to `VITEST_PACKAGE_TO_PATH`                    |
| pnpm override fails     | New plugin export needed       | Add to `createPluginExports()`                     |

---

## Technical Reference

### Build Flow

```
1. bundleVitest()              Copy vitest-dev dist/ -> dist/
2. copyVitestPackages()        Copy @vitest/* -> dist/@vitest/
3. convertTabsToSpaces()       Normalize formatting for patches
4. collectLeafDependencies()   Parse imports with oxc-parser
5. bundleLeafDeps()            Bundle chai, pathe, etc -> dist/vendor/
6. rewriteVitestImports()      Rewrite @vitest/*, vitest/*, vite
7. patchVitestPkgRootPaths()   Fix distRoot for relocated files
8. patchVitestBrowserPackage() Inject vendor-aliases plugin
9. patchBrowserProviderLocators() Fix browser-safe imports
10. Post-processing:
    - patchVendorPaths()
    - createBrowserCompatShim()
    - createModuleRunnerStub()   Browser-safe stub
    - createNodeEntry()          index-node.js with browser-provider
    - copyBrowserClientFiles()
    - createBrowserEntryFiles()  browser/ entry files at package root
    - createPluginExports()      dist/plugins/* for pnpm overrides
    - mergePackageJson()
    - validateExternalDeps()
```

### Output Structure

```
browser/                       # Entry files for ./browser export
├── context.js                 # Runtime guard (throws if not in browser)
└── context.d.ts               # Re-exports from dist/@vitest/browser/context.d.ts
dist/
├── @vitest/                    # Copied packages (browser/Node.js safe)
│   ├── runner/
│   ├── utils/
│   ├── spy/
│   ├── expect/
│   ├── snapshot/
│   ├── mocker/
│   ├── pretty-format/
│   ├── browser/
│   └── browser-playwright/
├── vendor/                     # Bundled leaf dependencies
│   ├── chai.mjs
│   ├── pathe.mjs
│   ├── tinyrainbow.mjs
│   ├── magic-string.mjs
│   ├── estree-walker.mjs
│   ├── why-is-node-running.mjs
│   └── vitest_*.mjs            # Browser stubs
├── plugins/                    # Shims for pnpm overrides
│   ├── runner.mjs
│   ├── utils.mjs
│   └── ... (33+ files)
├── chunks/                     # Vitest core chunks
├── client/                     # Browser client files
├── index.js                    # Browser-safe entry
├── index-node.js               # Node.js entry (includes browser-provider)
├── module-runner-stub.js       # Browser-safe module-runner
└── browser-compat.js           # @vitest/browser compatibility shim
```

### Browser/Node.js Separation

The critical design decision is maintaining separation between browser and Node.js code:

| Entry Point          | Used By               | Contains                          |
| -------------------- | --------------------- | --------------------------------- |
| `dist/index.js`      | Browser tests         | No Node.js-only code              |
| `dist/index-node.js` | Node.js (config, CLI) | Includes browser-provider exports |

This is achieved through:

1. Conditional exports in package.json (`"node": "./dist/index-node.js"`)
2. Browser-safe stubs for `module-runner`
3. Import rewriting to prevent Node.js code from being pulled into browser bundles
4. `vendor-aliases` plugin injection to resolve imports at runtime:
   - Handles `@vitest/*` imports → resolves to copied `dist/@vitest/` files
   - Handles `vitest/*` subpaths → resolves to dist files (enables `vitest/browser-playwright` usage)
   - Handles `vitest/browser-playwright`, `vitest/browser-webdriverio`, `vitest/browser-preview` → resolves to bundled browser providers
   - Handles `@voidzero-dev/vite-plus-test/*` subpaths → maps to equivalent vitest paths
   - Handles `vite-plus/test/*` subpaths → maps to equivalent vitest paths (CLI package)
   - Intercepts `vitest/browser`, `@voidzero-dev/vite-plus-test/browser`, `vite-plus/test/browser` → returns virtual module ID for BrowserContext plugin

### Key Constants

```typescript
// Packages copied to dist/@vitest/
const VITEST_PACKAGES_TO_COPY = [
  '@vitest/runner',
  '@vitest/utils',
  '@vitest/spy',
  '@vitest/expect',
  '@vitest/snapshot',
  '@vitest/mocker',
  '@vitest/pretty-format',
  '@vitest/browser',
  '@vitest/browser-playwright',
  '@vitest/browser-webdriverio',
  '@vitest/browser-preview',
];

// Packages that must NOT be bundled (from build.ts lines 131-158)
const EXTERNAL_BLOCKLIST = new Set([
  // Our own packages - resolved at runtime
  '@voidzero-dev/vite-plus-core',
  '@voidzero-dev/vite-plus-core/module-runner',
  'vite',
  'vitest',

  // Peer dependencies - consumers must provide these
  '@edge-runtime/vm',
  '@opentelemetry/api',
  'happy-dom',
  'jsdom',

  // Optional dependencies with bundling issues or native bindings
  'debug', // environment detection broken when bundled
  'playwright', // native bindings
  'webdriverio', // native bindings

  // Runtime deps (in package.json dependencies) - not bundled, resolved at install time
  'sirv',
  'ws',
  'pixelmatch',
  'pngjs',

  // MSW (Mock Service Worker) - optional peer dep of @vitest/mocker
  'msw',
  'msw/browser',
  'msw/core/http',
]);
```

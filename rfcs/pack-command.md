# RFC: `vp pack` Command

## Summary

`vp pack` bundles TypeScript/JavaScript libraries using tsdown (Rolldown-powered bundler). Configured via `vite.config.ts` under the `pack` key.

## Motivation

Unified library bundling integrated into the Vite+ toolchain, replacing standalone `tsdown` CLI usage. A single config file (`vite.config.ts`) manages all tools — dev server, build, test, lint, and now pack.

### Current Pain Points

```bash
# Standalone tsdown requires its own config file
npx tsdown src/index.ts --format esm --dts

# Separate config from the rest of the Vite ecosystem
# tsdown.config.ts vs vite.config.ts — fragmented tooling
```

### Proposed Solution

```bash
# Integrated into vp CLI
vp pack src/index.ts --format esm --dts

# Config lives in vite.config.ts alongside everything else
# vite.config.ts
export default {
  pack: { entry: 'src/index.ts', format: ['esm', 'cjs'], dts: true }
}
```

## Command Syntax

```bash
vp pack [...files] [options]
```

### Usage Examples

```bash
# Bundle with defaults (ESM, node platform)
vp pack src/index.ts

# Multiple formats
vp pack src/index.ts --format esm --format cjs

# With declaration files
vp pack src/index.ts --dts

# Watch mode with success hook
vp pack src/index.ts --watch --on-success 'node dist/index.mjs'

# Workspace mode
vp pack --workspace --filter my-lib

# Bundle as executable (experimental, Node.js >= 25.7.0)
vp pack src/cli.ts --exe
```

## CLI Options

### Input

- `[...files]` — Entry files to bundle
- `--config-loader <loader>` — Config loader to use: `auto`, `native`, `unrun` (default: `auto`)
- `--no-config` — Disable config file
- `--from-vite [vitest]` — Reuse config from Vite or Vitest

### Output

- `-f, --format <format>` — Bundle format: `esm`, `cjs`, `iife`, `umd` (default: `esm`)
- `-d, --out-dir <dir>` — Output directory (default: `dist`)
- `--clean` — Clean output directory, `--no-clean` to disable
- `--sourcemap` — Generate source map (default: `false`)
- `--shims` — Enable CJS and ESM shims (default: `false`)
- `--minify` — Minify output

### Declaration Files

- `--dts` — Generate `.d.ts` files

### Platform & Target

- `--platform <platform>` — Target platform: `node`, `browser`, `neutral` (default: `node`)
- `--target <target>` — Bundle target, e.g., `es2015`, `esnext`

### Dependencies

- `--deps.never-bundle <module>` — Mark dependencies as external
- `--treeshake` — Tree-shake bundle (default: `true`)

### Quality Checks

- `--publint` — Enable publint (default: `false`)
- `--attw` — Enable Are the types wrong integration (default: `false`)
- `--unused` — Enable unused dependencies check (default: `false`)

### Watch Mode

- `-w, --watch [path]` — Watch mode
- `--ignore-watch <path>` — Ignore custom paths in watch mode
- `--on-success <command>` — Command to run on success

### Environment

- `--env.* <value>` — Define compile-time env variables
- `--env-file <file>` — Load environment variables from a file (variables in `--env` take precedence)
- `--env-prefix <prefix>` — Prefix for env variables to inject into the bundle (default: `VITE_PACK_,TSDOWN_`)

### Workspace

- `-W, --workspace [dir]` — Enable workspace mode
- `-F, --filter <pattern>` — Filter configs (cwd or name), e.g., `/pkg-name$/` or `pkg-name`

### Other

- `--copy <dir>` — Copy files to output dir
- `--public-dir <dir>` — Alias for `--copy` (deprecated)
- `--tsconfig <tsconfig>` — Set tsconfig path
- `--unbundle` — Unbundle mode
- `--report` — Size report (default: `true`)
- `--exports` — Generate export-related metadata for package.json (experimental)
- `--debug [feat]` — Show debug logs
- `-l, --logLevel <level>` — Set log level: `info`, `warn`, `error`, `silent`
- `--fail-on-warn` — Fail on warnings (default: `true`)
- `--no-write` — Disable writing files to disk, incompatible with watch mode
- `--devtools` — Enable devtools integration

### Executable (Experimental)

- `--exe` — Bundle as Node.js Single Executable Application (SEA)
  - Requires Node.js >= 25.7.0
  - Single entry point only
  - Defaults to ESM format, DTS generation disabled by default
  - On macOS, applies ad-hoc codesigning automatically

## Configuration

Config is specified in `vite.config.ts` under the `pack` key:

```ts
// Single config
export default {
  pack: {
    entry: 'src/index.ts',
    format: ['esm', 'cjs'],
    dts: true,
  },
};

// Array for multiple configs
export default {
  pack: [
    { entry: 'src/index.ts', format: ['esm'], dts: true },
    { entry: 'src/cli.ts', format: ['cjs'] },
  ],
};
```

CLI flags override config file values. When both are provided, CLI flags take precedence.

## Architecture

### Command Dispatch

```
Global CLI (Rust) ─── Category C delegation ───▸ Local CLI (pack-bin.ts) ───▸ tsdown
```

1. **Global CLI** (`crates/vite_global_cli/src/cli.rs`): The `Pack` command variant uses `trailing_var_arg` to capture all arguments, then unconditionally delegates to the local CLI.
2. **Local CLI** (`packages/cli/src/pack-bin.ts`): Parses CLI options with `cac`, resolves config from `vite.config.ts`, and calls tsdown's `resolveUserConfig` + `buildWithConfigs`.
3. **tsdown**: Handles all bundling logic, including the new SEA/exe feature.

### Config Resolution

```
vite.config.ts (pack key) ──▸ merge with CLI flags ──▸ resolveUserConfig() ──▸ buildWithConfigs()
```

The local CLI:

1. Resolves Vite config via `resolveConfig()` to find `vite.config.ts`
2. Reads the `pack` key (object or array)
3. Merges each pack config with CLI flags (CLI wins)
4. Passes through to tsdown's `resolveUserConfig` for full resolution
5. Calls `buildWithConfigs` with all resolved configs

### Environment Variable Prefix

- Default prefix: `VITE_PACK_` (primary) and `TSDOWN_` (migration compatibility)
- Variables matching these prefixes are injected into the bundle at compile time
- Customizable via `--env-prefix`

### tsdown Integration

tsdown is bundled inside `@voidzero-dev/vite-plus-core/pack`:

- `packages/core/build.ts` bundles tsdown's JS, CJS dependencies, and types
- `packages/core/package.json` tracks `bundledVersions.tsdown`
- Re-exported via `packages/cli/src/pack.ts`

## `--exe` Feature (Experimental)

The `--exe` flag bundles the output as a Node.js Single Executable Application (SEA).

### Requirements

- Node.js >= 25.7.0 (uses the `node --build-sea` API)
- Single entry point only

### Behavior

When `--exe` is passed:

1. tsdown defaults to ESM format
2. DTS generation is disabled by default
3. The bundle is embedded into a Node.js SEA blob
4. On macOS, ad-hoc codesigning is applied automatically
5. The resulting executable is a standalone binary

### Error Handling

If Node.js version is too old:

```
Node.js version v22.22.0 does not support `exe` option. Please upgrade to Node.js 25.7.0 or later.
```

## Relationship with `vp pm pack`

These are distinct commands:

| Command      | Purpose                           | Output              |
| ------------ | --------------------------------- | ------------------- |
| `vp pack`    | Library bundling via tsdown       | `dist/` directory   |
| `vp pm pack` | Tarball creation via npm/pnpm/bun | `.tgz` package file |

**Note:** For tarball creation, bun uses `bun pm pack` (not `bun pack`). It supports `--destination` and `--dry-run` flags. See the [pm-command-group RFC](./pm-command-group.md) for the full command mapping.

## Design Decisions

### 1. Config in `vite.config.ts` (Not `tsdown.config.ts`)

**Decision**: Pack config lives under the `pack` key in `vite.config.ts`.

**Rationale**:

- Single config file for the entire Vite+ toolchain
- Consistent with how `vp build`, `vp test`, etc. are configured
- Reduces config file proliferation in projects

### 2. `VITE_PACK_` Env Prefix (+ `TSDOWN_` for Migration)

**Decision**: Default env prefix is `VITE_PACK_` with `TSDOWN_` as a migration-compatible fallback.

**Rationale**:

- `VITE_PACK_` follows Vite+ naming conventions
- `TSDOWN_` ensures projects migrating from standalone tsdown continue to work
- Users can override with `--env-prefix`

### 3. tsdown Bundled Inside Core

**Decision**: tsdown is bundled inside `@voidzero-dev/vite-plus-core/pack` rather than used as a direct dependency.

**Rationale**:

- Ensures consistent tsdown version across all vite-plus users
- Avoids version conflicts in monorepos
- The core build process bundles JS, CJS deps, and types together

### 4. Category C Delegation

**Decision**: The global CLI unconditionally delegates to the local CLI for `pack`.

**Rationale**:

- Pack requires project context (config file, dependencies, etc.)
- Follows the same pattern as `build`, `test`, `lint`
- No meaningful global-only behavior for bundling

## CLI Help Output

```bash
$ vp pack -h
vp pack

Usage:
  $ vp pack [...files]

Commands:
  [...files]  Bundle files

Options:
  --config-loader <loader>  Config loader to use: auto, native, unrun (default: auto)
  --no-config               Disable config file
  -f, --format <format>     Bundle format: esm, cjs, iife, umd (default: esm)
  --clean                   Clean output directory, --no-clean to disable
  --deps.never-bundle <module>  Mark dependencies as external
  --minify                  Minify output
  --devtools                Enable devtools integration
  --debug [feat]            Show debug logs
  --target <target>         Bundle target, e.g "es2015", "esnext"
  -l, --logLevel <level>    Set log level: info, warn, error, silent
  --fail-on-warn            Fail on warnings (default: true)
  --no-write                Disable writing files to disk, incompatible with watch mode
  -d, --out-dir <dir>       Output directory (default: dist)
  --treeshake               Tree-shake bundle (default: true)
  --sourcemap               Generate source map (default: false)
  --shims                   Enable cjs and esm shims (default: false)
  --platform <platform>     Target platform (default: node)
  --dts                     Generate dts files
  --publint                 Enable publint (default: false)
  --attw                    Enable Are the types wrong integration (default: false)
  --unused                  Enable unused dependencies check (default: false)
  -w, --watch [path]        Watch mode
  --ignore-watch <path>     Ignore custom paths in watch mode
  --from-vite [vitest]      Reuse config from Vite or Vitest
  --report                  Size report (default: true)
  --env.* <value>           Define compile-time env variables
  --env-file <file>         Load environment variables from a file
  --env-prefix <prefix>     Prefix for env variables to inject into the bundle
  --on-success <command>    Command to run on success
  --copy <dir>              Copy files to output dir
  --public-dir <dir>        Alias for --copy, deprecated
  --tsconfig <tsconfig>     Set tsconfig path
  --unbundle                Unbundle mode
  -W, --workspace [dir]     Enable workspace mode
  -F, --filter <pattern>    Filter configs (cwd or name)
  --exports                 Generate export-related metadata for package.json (experimental)
  --exe                     Bundle as executable using Node.js SEA (experimental)
  -h, --help                Display this message
```

## Snap Tests

### Local CLI Test: `command-pack`

**Location**: `packages/cli/snap-tests/command-pack/`

Tests `vp pack -h` (help output includes all options including `--exe`) and `vp run pack` (build and cache hit).

### Local CLI Test: `command-pack-exe`

**Location**: `packages/cli/snap-tests/command-pack-exe/`

Tests `vp pack src/index.ts --exe` error behavior when Node.js version is below 25.7.0.

## Backward Compatibility

This RFC documents an existing command with no breaking changes:

- All existing `vp pack` options continue to work
- The new `--exe` flag is purely additive
- Config format in `vite.config.ts` is unchanged

## Exe Advanced Configuration

### Programmatic `ExeOptions`

The `exe` option accepts an object for advanced configuration:

```ts
export default {
  pack: {
    entry: 'src/cli.ts',
    exe: {
      seaConfig: {
        /* Node.js SEA config overrides */
      },
      fileName: 'my-cli',
      targets: [
        { platform: 'linux', arch: 'x64', nodeVersion: '25.7.0' },
        { platform: 'darwin', arch: 'arm64' },
      ],
    },
  },
};
```

### Cross-Platform Executable Building

Cross-platform builds are supported via the `@tsdown/exe` package (optional peer dependency). The `targets` option accepts an array of `{ platform, arch, nodeVersion }` objects to build executables for different platforms from a single host.

## Conclusion

`vp pack` integrates tsdown-powered library bundling into the Vite+ toolchain. By using `vite.config.ts` for configuration and following the Category C delegation pattern, it provides a consistent developer experience alongside `vp build`, `vp test`, and `vp lint`. The new `--exe` flag (experimental) enables bundling as standalone Node.js executables via the SEA API.

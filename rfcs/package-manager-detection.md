# RFC: Package Manager Detection

## Summary

Document how Vite+ determines which package manager (pnpm/yarn/npm/bun) a project uses. This detection runs automatically before package management commands (`vp install`, `vp add`, `vp remove`, etc.) and drives PM-specific behavior including command translation, lockfile handling, workspace configuration, and matching package-manager shims.

## Detection Algorithm

Vite+ uses a strict priority-ordered algorithm to detect the package manager. The first match wins.

### Priority 1: `packageManager` field in `package.json`

The highest-priority signal. If the root `package.json` contains a `packageManager` field, it is used unconditionally.

```json
{
  "packageManager": "pnpm@10.19.0"
}
```

**Format**: `<name>@<semver>[+<hash>]`

- `name` must be one of: `pnpm`, `yarn`, `npm`, `bun`
- `semver` must be valid (e.g., `10.19.0`, `4.0.0`)
- Optional integrity hash suffix: `pnpm@10.0.0+sha512.abc123...` (see [Integrity Hashes](#integrity-hashes))

**Errors**:

- Invalid semver → `PackageManagerVersionInvalid` error
- Unknown name → `UnsupportedPackageManager` error

**Reference**: [Node.js Corepack packageManager field](https://nodejs.org/api/packages.html#packagemanager)

The explicit field also controls matching package-manager shims, including aliases generated for that manager. If a project declares `packageManager: "npm@11.14.0"`, the `npm` and `npx` shims run npm 11.14.0. Other aliases follow the same rule: `pnpm`/`pnpx`, `yarn`/`yarnpkg`, and `bun`/`bunx`. If the project declares `pnpm`, `yarn`, or `bun`, invoking `npm` still runs npm; Vite+ never translates one package-manager shim command into another.

When `devEngines.packageManager` is also declared, the `packageManager` field still drives selection, but Vite+ warns when the field's name or version does not satisfy the devEngines constraint (this warning becomes a hard error in a future release; npm already errors in this situation). See [RFC: devEngines Support](./dev-engines.md).

### Priority 2: `devEngines.packageManager` field in `package.json`

If there is no `packageManager` field, Vite+ checks `devEngines.packageManager`, following the [devEngines spec](https://github.com/openjs-foundation/package-metadata-interoperability-working-group/blob/main/devengines-field-proposal.md):

```json
{
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "^11.0.0",
      "onFail": "download"
    }
  }
}
```

- Accepts a single object or an array of objects; entries are evaluated in order and the first entry with a supported `name` wins.
- `name` must be one of `pnpm`, `yarn`, `npm`, `bun`. Unsupported names are skipped in array form. When no entry names a supported package manager, the effective `onFail` of the last entry decides: `ignore`/`warn` continue down the detection chain, `error`/`download` fail with a clear message.
- `version` may be exact, a semver range, or absent (any version satisfies). Ranges resolve to an already-downloaded satisfying version when possible, otherwise to the latest satisfying version from the npm registry (fetched as the abbreviated metadata document). Prereleases are excluded unless the range itself contains a prerelease marker and no stable version satisfies it.
- A range source is never frozen into an exact `packageManager` field; the range stays the source of truth.
- `onFail` is otherwise parsed and preserved but not yet acted on: a selected (supported) entry whose version cannot be resolved or downloaded surfaces an error rather than falling back. See the RFC's [Deferred / Future Work](./dev-engines.md#deferred--future-work).

See [RFC: devEngines Support](./dev-engines.md) for the full semantics (conflict handling, doctor checks, and the deferred `onFail` matrix).

### Priority 3: Lockfiles

If neither `packageManager` nor `devEngines.packageManager` is found, Vite+ checks for lockfiles in the workspace root. Checked in this order:

| File                  | Detected PM | Notes                            |
| --------------------- | ----------- | -------------------------------- |
| `pnpm-workspace.yaml` | pnpm        | Workspace definition file        |
| `pnpm-lock.yaml`      | pnpm        | Lockfile                         |
| `yarn.lock`           | yarn        | Lockfile                         |
| `.yarnrc.yml`         | yarn        | Yarn Berry (v2+) configuration   |
| `package-lock.json`   | npm         | Lockfile                         |
| `bun.lock`            | bun         | Text-format lockfile (preferred) |
| `bun.lockb`           | bun         | Binary-format lockfile (legacy)  |

When detected from lockfiles, version is set to `"latest"` (resolved during download).

### Priority 4: Configuration files

Lower-priority config files that indicate a package manager:

| File              | Detected PM | Notes                                       |
| ----------------- | ----------- | ------------------------------------------- |
| `.pnpmfile.cjs`   | pnpm        | [pnpm hooks](https://pnpm.io/pnpmfile)      |
| `pnpmfile.cjs`    | pnpm        | Legacy format (pnpm v5.x)                   |
| `bunfig.toml`     | bun         | [Bun configuration](https://bun.sh/docs/pm) |
| `yarn.config.cjs` | yarn        | Yarn Berry (v2+) configuration              |

### Priority 5: Explicit default

If a caller provides a default package manager type (used internally by some code paths), that default is used with version `"latest"`.

### Priority 6: Interactive selection

If no signals are detected and no default is provided, the behavior depends on the environment:

#### CI environment

Checks for common CI environment variables:

- `CI`, `CONTINUOUS_INTEGRATION`, `GITHUB_ACTIONS`, `GITLAB_CI`, `CIRCLECI`, `TRAVIS`, `JENKINS_URL`, `BUILDKITE`, `DRONE`, `CODEBUILD_BUILD_ID` (AWS CodeBuild), `TF_BUILD` (Azure Pipelines)

**Result**: Auto-selects `pnpm` without prompting.

#### Non-interactive terminal

If stdin is not a TTY (piped input, non-interactive shell):

**Result**: Auto-selects `pnpm` without prompting.

#### Interactive terminal

Displays a keyboard-navigable menu:

```
No package manager detected. Please select one:
   Use ↑↓ arrows to navigate, Enter to select, 1-4 for quick selection

  ▶ [1] pnpm (recommended) ←
    [2] npm
    [3] yarn
    [4] bun
```

If the interactive menu fails (terminal compatibility issues), falls back to a simple text prompt:

```
No package manager detected. Please select one:
────────────────────────────────────────────────
  [1] pnpm (recommended)
  [2] npm
  [3] yarn
  [4] bun

Enter your choice (1-4) [default: 1]:
```

## CLI Flag: `--package-manager`

The `vp create` command supports a `--package-manager` flag for explicitly specifying the package manager:

```bash
vp create vite:monorepo --no-interactive --package-manager bun
```

**Resolution priority for `vp create`**:

1. Any package manager detected for an existing monorepo (from manifest fields, workspace files, lockfiles, or package-manager configuration)
2. `--package-manager` CLI flag
3. Package manager detected from a non-monorepo ancestor
4. Interactive prompt / auto-default (pnpm)

This ensures monorepo consistency while allowing standalone projects to override ambient detection explicitly.

## Non-Mutating Resolution

Detection and download never rewrite `package.json`. A `devEngines.packageManager` range remains the source of truth, while lockfile, config, and interactive detection resolve a managed package manager for the current command without adding a manifest field.

Projects that require a deterministic declaration can pin it explicitly with `vp env pin <package-manager>@<version>`. Commands that modify dependencies, including `vp install` and `vp add`, require an existing `package.json` instead of creating one automatically.

## Version Resolution

| Detection method                              | Version used                                                                                             |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `packageManager` field                        | Exact version from field (e.g., `10.19.0`)                                                               |
| `devEngines.packageManager` (exact version)   | Exact version from field                                                                                 |
| `devEngines.packageManager` (range or absent) | Highest already-downloaded satisfying version, otherwise latest satisfying version from the npm registry |
| Lockfile/config detection                     | `"latest"`: resolved to latest stable version from npm registry                                          |
| Interactive selection                         | `"latest"`: resolved to latest stable version from npm registry                                          |

**Special cases**:

- **yarn ≥ 2.0.0**: Downloads from `@yarnpkg/cli-dist` instead of the `yarn` npm package, and extracts only `bin/yarn.js`. Every 2.x prerelease counts as Yarn 2 or later; see [the Yarn 2 boundary](#the-yarn-2-boundary).
- **bun**: Downloads platform-specific native binary from `@oven/bun-{os}-{arch}` (including musl variants for Alpine Linux)

## Integrity Hashes

A `packageManager` field can carry an integrity hash: `yarn@4.17.1+sha512.ccbf…`. `corepack use` writes that suffix. Vite+ hashes the same artifact as Corepack, so one pin works under both tools.

| Package manager              | What the declared hash covers                       | What Vite+ also verifies                                   |
| ---------------------------- | --------------------------------------------------- | ---------------------------------------------------------- |
| Yarn 2 and later             | the extracted CLI, `bin/yarn.js`                    | —                                                          |
| npm, pnpm ≤ 11, Yarn Classic | the npm package tarball                             | —                                                          |
| pnpm ≥ 12                    | the main `pnpm` tarball                             | the platform package against the registry `dist.integrity` |
| bun                          | the main `bun` tarball, which Vite+ never downloads | the platform package against the registry `dist.integrity` |

Yarn 2 and later is the exception because Corepack installs Berry from a single file, `repo.yarnpkg.com/<version>/packages/yarnpkg-cli/bin/yarn.js`, and hashes that file. Vite+ downloads the `@yarnpkg/cli-dist` tarball instead, so it extracts `bin/yarn.js` and hashes that entry. The bytes are the same; only the basis differs. Vite+ hashed the tarball before, which made a pin written by `corepack use` fail (issue #2209).

That pin covers one file inside an otherwise unauthenticated archive, so Vite+ writes only that entry to disk. No other archive entry reaches the install directory, and an archive-controlled path or symlink cannot escape it.

### When Vite+ verifies a pin

Vite+ hashes the artifact when it downloads it, and records the verified pin beside the install in `<version>/.verified-pin`. A later command compares its own pin against that record:

- The pins match. The command uses the cache and reads no further.
- The pins differ, or the record is missing. Vite+ hashes the cached CLI once, then rewrites the record.
- The hash disagrees with the pin. The command stops with `Hash mismatch for <name>@<version>`, and the message names the artifact the hash covers.

Vite+ does not read the CLI again on every command. Corepack gives the same guarantee: it reads its own `.corepack` record and returns. The trust boundary is write access to `$VP_HOME`, which also holds the `vp` binary, the generated shims, and the managed Node.js runtime.

An integrity failure stops the command that needs the package manager, including `vp run` and `vp exec`. Those commands otherwise continue when the managed package manager is missing, for example with no network or an unknown version. A swallowed integrity failure would surface later as "command not found".

### The Yarn 2 boundary

Corepack splits Yarn at 2.0.0 and matches that range with `satisfiesWithPrereleases`, which drops the prerelease tag before it compares. Every 2.x prerelease is therefore a Berry version to Corepack. Vite+ compares the major number alone and agrees: `yarn@4.0.0-rc.53` resolves from `@yarnpkg/cli-dist`. A `>=2.0.0` semver range would exclude that version and send it to the Yarn Classic package, which never published it.

## Workspace and Monorepo Detection

Workspace detection determines `is_monorepo` based on:

- `pnpm-workspace.yaml` → monorepo (pnpm)
- `package.json` with `workspaces` field → monorepo (npm/yarn/bun)

The package manager type and monorepo status together drive:

- Which lockfile patterns to watch for cache invalidation
- Whether catalog support is available (pnpm, yarn, bun — not npm)
- How workspace filters (`--filter`) are translated

## Detection Signals Summary

### Per package manager

| Package Manager | Lockfiles               | Config Files                                           | Fields                                        |
| --------------- | ----------------------- | ------------------------------------------------------ | --------------------------------------------- |
| pnpm            | `pnpm-lock.yaml`        | `pnpm-workspace.yaml`, `.pnpmfile.cjs`, `pnpmfile.cjs` | `packageManager`, `devEngines.packageManager` |
| yarn            | `yarn.lock`             | `.yarnrc.yml`, `.yarnrc`, `yarn.config.cjs`            | `packageManager`, `devEngines.packageManager` |
| npm             | `package-lock.json`     | —                                                      | `packageManager`, `devEngines.packageManager` |
| bun             | `bun.lock`, `bun.lockb` | `bunfig.toml`                                          | `packageManager`, `devEngines.packageManager` |

### Cache invalidation (fingerprint ignores)

Each package manager has specific files that trigger cache invalidation when changed:

| Package Manager | Watched Files                                                                        |
| --------------- | ------------------------------------------------------------------------------------ |
| pnpm            | `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `.pnpmfile.cjs`, `pnpmfile.cjs`, `.pnp.cjs` |
| yarn            | `.yarnrc`, `.yarnrc.yml`, `yarn.config.cjs`, `yarn.lock`, `.yarn/**/*`, `.pnp.cjs`   |
| npm             | `package-lock.json`, `npm-shrinkwrap.json`                                           |
| bun             | `bun.lock`, `bun.lockb`, `bunfig.toml`                                               |
| All             | `**/package.json`, `.npmrc`                                                          |

## Implementation

### Rust (core detection)

- **File**: `crates/vp_pm_cli/src/package_manager.rs`
- **Function**: `get_package_manager_type_and_version()` — priority-ordered detection
- **Function**: `prompt_package_manager_selection()` — CI/TTY/interactive fallback
- **Function**: `download_package_manager()` — download, hash, and record the verified pin
- **Function**: `ensure_package_manager_bin()` — resolve the executable, shared with the global shim
- **Function**: `verify_cached_cli_hash()` — compare a pin against the recorded pin
- **Enum**: `PackageManagerType` — `Pnpm`, `Yarn`, `Npm`, `Bun`

### TypeScript (CLI integration)

- **File**: `packages/cli/src/utils/workspace.ts` — `detectWorkspace()` wraps NAPI binding
- **File**: `packages/cli/src/utils/prompts.ts` — `selectPackageManager()` for non-interactive default
- **File**: `packages/cli/src/create/bin.ts` — `--package-manager` flag handling

### NAPI binding (bridge)

- **File**: `packages/cli/binding/src/package_manager.rs` — `detectWorkspace()` exports to JS

## Future Enhancements

### Multiple lockfile conflict resolution

Currently, if multiple lockfiles exist (e.g., both `pnpm-lock.yaml` and `package-lock.json`), the first one found in priority order wins silently. A future enhancement could warn about conflicting lockfiles and suggest cleanup.

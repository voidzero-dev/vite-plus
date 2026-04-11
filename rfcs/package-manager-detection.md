# RFC: Package Manager Detection

## Summary

Document how Vite+ determines which package manager (pnpm/yarn/npm/bun) a project uses. This detection runs automatically before any package management command (`vp install`, `vp add`, `vp remove`, etc.) and drives all PM-specific behavior including command translation, lockfile handling, and workspace configuration.

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
- Optional hash suffix: `pnpm@10.0.0+sha512.abc123...`

**Errors**:

- Invalid semver → `PackageManagerVersionInvalid` error
- Unknown name → `UnsupportedPackageManager` error

**Reference**: [Node.js Corepack packageManager field](https://nodejs.org/api/packages.html#packagemanager)

### Priority 2: Lockfiles

If no `packageManager` field is found, Vite+ checks for lockfiles in the workspace root. Checked in this order:

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

### Priority 3: Configuration files

Lower-priority config files that indicate a package manager:

| File              | Detected PM | Notes                                       |
| ----------------- | ----------- | ------------------------------------------- |
| `.pnpmfile.cjs`   | pnpm        | [pnpm hooks](https://pnpm.io/pnpmfile)      |
| `pnpmfile.cjs`    | pnpm        | Legacy format (pnpm v5.x)                   |
| `bunfig.toml`     | bun         | [Bun configuration](https://bun.sh/docs/pm) |
| `yarn.config.cjs` | yarn        | Yarn Berry (v2+) configuration              |

### Priority 4: Explicit default

If a caller provides a default package manager type (used internally by some code paths), that default is used with version `"latest"`.

### Priority 5: Interactive selection

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

1. Detected workspace `packageManager` field (existing monorepo takes precedence)
2. `--package-manager` CLI flag
3. Interactive prompt / auto-default (pnpm)

This ensures monorepo consistency: if you run `vp create` inside an existing workspace that already has a `packageManager` field, the workspace setting wins over the CLI flag.

## Auto-Update Behavior

After detection and download, Vite+ automatically writes the resolved package manager version to the `packageManager` field in `package.json`. This ensures:

- Future runs use the exact version (Priority 1 match)
- Team members get consistent versions
- CI environments use deterministic versions

## Version Resolution

| Detection method          | Version used                                                     |
| ------------------------- | ---------------------------------------------------------------- |
| `packageManager` field    | Exact version from field (e.g., `10.19.0`)                       |
| Lockfile/config detection | `"latest"` — resolved to latest stable version from npm registry |
| Interactive selection     | `"latest"` — resolved to latest stable version from npm registry |

**Special cases**:

- **yarn ≥ 2.0.0**: Downloads from `@yarnpkg/cli-dist` instead of `yarn` npm package
- **bun**: Downloads platform-specific native binary from `@oven/bun-{os}-{arch}` (including musl variants for Alpine Linux)

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

| Package Manager | Lockfiles               | Config Files                                           | Field            |
| --------------- | ----------------------- | ------------------------------------------------------ | ---------------- |
| pnpm            | `pnpm-lock.yaml`        | `pnpm-workspace.yaml`, `.pnpmfile.cjs`, `pnpmfile.cjs` | `packageManager` |
| yarn            | `yarn.lock`             | `.yarnrc.yml`, `.yarnrc`, `yarn.config.cjs`            | `packageManager` |
| npm             | `package-lock.json`     | —                                                      | `packageManager` |
| bun             | `bun.lock`, `bun.lockb` | `bunfig.toml`                                          | `packageManager` |

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

- **File**: `crates/vite_install/src/package_manager.rs`
- **Function**: `get_package_manager_type_and_version()` — priority-ordered detection
- **Function**: `prompt_package_manager_selection()` — CI/TTY/interactive fallback
- **Enum**: `PackageManagerType` — `Pnpm`, `Yarn`, `Npm`, `Bun`

### TypeScript (CLI integration)

- **File**: `packages/cli/src/utils/workspace.ts` — `detectWorkspace()` wraps NAPI binding
- **File**: `packages/cli/src/utils/prompts.ts` — `selectPackageManager()` for non-interactive default
- **File**: `packages/cli/src/create/bin.ts` — `--package-manager` flag handling

### NAPI binding (bridge)

- **File**: `packages/cli/binding/src/package_manager.rs` — `detectWorkspace()` exports to JS

## Future Enhancements

### `devEngines.packageManager` field

Support the [Node.js `devEngines` field](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devengines) for package manager constraints:

```json
{
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": ">=10.0.0"
    }
  }
}
```

This would be checked between Priority 1 (`packageManager` field) and Priority 2 (lockfiles). It specifies a constraint rather than an exact version, so it would be combined with other signals.

### Multiple lockfile conflict resolution

Currently, if multiple lockfiles exist (e.g., both `pnpm-lock.yaml` and `package-lock.json`), the first one found in priority order wins silently. A future enhancement could warn about conflicting lockfiles and suggest cleanup.

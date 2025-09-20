# `vite install`

## Overview

`vite install` will automatically select the most suitable packageManager based on the current directory configuration, then run its install command, and generate a new fingerprint.

Currently only pnpm, yarn, and npm are supported. More package managers like bun, [vlt](https://docs.vlt.sh/cli/commands/install), lerna*,* etc. will be considered in the future.

> Assume the current running environment has at least `node` installed

## Detect workspace root directory

User maybe run `vite install` in a workspace subdirectory, we need to relocate to the workspace root directory to run.

### monorepo

Search for workspace information from the current directory. If not found, search in the parent directory, all the way up to the operating system root directory.

Workspace root detection priority:

- `pnpm-workspace.yaml` file exists
- `package.json` file exists and `workspaces` field exists on it

If no workspace identification information is found, it indicates that the current project is not a monorepo and should be treated as a normal repo.

Examples

/path/to/pnpm-worksapce.yaml

```yaml
packages:
  - "packages/*"
  - "apps/*"
  - "!**/test/**"
```

/path/to/package.json

```json
{
  "name": "my-monorepo",
  "version": "1.0.0",
  "private": true,
  "workspaces": [
    "packages/*"
  ]
}
```

### normal repo

Starting from the current directory, recursively search upwards to find the directory containing the `package.json` file as the workspace root; if not found, use the current directory as the workspace root.

## Detect package manager

Package manager detection priority:

- `packageManager` field in `package.json` , got `{pm}@{version}`
- `devEngines.packageManager` field in `package.json` (WIP)
- `pnpm-lock.yaml` or `pnpm-workspace.yaml` file exists ⇒ `pnpm@latest`
- `yarn.lock` or `.yarnrc.yml` file exists ⇒ `yarn@latest`
- `package-lock.json` exists ⇒ `npm@latest`
- `pnpmfile.cjs` exists ⇒ `pnpm@latest`
- `yarn.config.cjs` exists ⇒ `yarn@latest`, support in yarn 2.0+
- List pnpm/yarn/npm to let user choice and recommend user to use `pnpm@latest` by default

If `packageManager` filed not exists on package.json, it will auto filled correctly after install run.

## Install package manager

If the detected package manager does not exist, we will download and use it locally.

We will install the package manager in a local directory without polluting the user's global installation.

Also need to [make shims](https://github.com/nodejs/corepack/blob/main/mkshims.ts) like corepack does.

## Run `pm install` and collect fingerprint

When running `pm install`, it will use the same approach as vite task to collect fingerprints. Since the install process generates a huge node_modules, specific special filtering will be applied.

The ignored paths are as follows:

- All paths under the node_modules directory, but will retain the first-level filename list of node_modules as a fingerprint
- Paths that are not within the `cwd` scope

## Manual execution

If the user want their modifications to be cleaned up by a fresh install, they should manually run `vite install`. It will ignore cache checks and re-execute `pm install`, generating a new fingerprint.

## Auto execution

The `vite install` task will be auto-executed or quickly skipped at the beginning of any other vite+ command, eliminating the need for the user to run it manually in most cases.

### No replay when cache hit

The auto-executed `vite install` task will not replay stdout after hitting the cache, to avoid interfering with the real execution of the command.

## Architecture

### Install Execution Flow

```
┌──────────────────────────────────────────────────────────────┐
│                    Install Execution Flow                    │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Install Request                                          │
│  ──────────────────                                          │
│    vite install [args]                                       │
│         │                                                    │
│         ▼                                                    │
│  2. Workspace Detection                                      │
│  ──────────────────────                                      │
│    • Check pnpm-workspace.yaml                               │
│    • Check package.json#workspaces                           │
│    • Search upward for package.json                          │
│         │                                                    │
│         ▼                                                    │
│  3. Package Manager Detection                                │
│  ────────────────────────────                                │
│    ┌──────────────────────┬────────────────┐                 │
│    │ packageManager field │   Lock Files   │                 │
│    └──────────┬───────────┴────────┬───────┘                 │
│               │                     │                        │
│               ▼                     ▼                        │
│  4a. Parse pm@version    4b. Infer from locks                │
│  ────────────────────    ────────────────────                │
│    • pnpm@8.15.0           • pnpm-lock.yaml → pnpm           │
│    • yarn@4.0.0            • yarn.lock → yarn                │
│    • npm@10.0.0            • package-lock.json → npm         │
│               │                     │                        │
│               └──────────┬──────────┘                        │
│                          │                                   │
│                          ▼                                   │
│  5. Package Manager Installation                             │
│  ────────────────────────────────                            │
│    • Check local cache for pm@version                        │
│    • Download if missing (with retry)                        │
│    • Extract and create shims                                │
│    • Update PATH environment                                 │
│         │                                                    │
│         ▼                                                    │
│  6. Execute Install Command                                  │
│  ──────────────────────────                                  │
│    • Run: {pm} install [args]                                │
│    • Monitor with fspy                                       │
│    • Capture stdout/stderr                                   │
│         │                                                    │
│         ▼                                                    │
│  7. Fingerprint Collection                                   │
│  ─────────────────────────                                   │
│    • Hash package.json                                       │
│    • Hash lock file                                          │
│    • List node_modules/* (names only)                        │
│         │                                                    │
│         ▼                                                    │
│  8. Cache Storage                                            │
│  ────────────────                                            │
│    • Save fingerprint                                        │
│    • Store outputs                                           │
│    • Update packageManager field                             │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Package Manager Resolution

```
┌──────────────────────────────────────────────────────────────┐
│                 Package Manager Resolution                   │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Priority Order:                                             │
│  ──────────────                                              │
│    1. package.json#packageManager                            │
│    2. pnpm-workspace.yaml exists                             │
│    3. Lock file detection                                    │
│    4. User selection (interactive)                           │
│         │                                                    │
│         ▼                                                    │
│  Detection Flow:                                             │
│  ──────────────                                              │
│                                                              │
│    package.json                                              │
│         │                                                    │
│         ├─► packageManager: "pnpm@8.15.0"                    │
│         │   └─► Use exact version                            │
│         │                                                    │
│         └─► No packageManager field                          │
│             │                                                │
│             ├─► pnpm-workspace.yaml exists?                  │
│             │   └─► pnpm@latest                              │
│             │                                                │
│             ├─► pnpm-lock.yaml exists?                       │
│             │   └─► pnpm@latest                              │
│             │                                                │
│             ├─► yarn.lock exists?                            │
│             │   └─► yarn@latest                              │
│             │                                                │
│             ├─► package-lock.json exists?                    │
│             │   └─► npm@latest                               │
│             │                                                │
│             └─► No indicators found                          │
│                 │                                            │
│                 ├─► CI environment?                          │
│                 │   └─► Auto-select pnpm                     │
│                 │                                            │
│                 ├─► Non-TTY?                                 │
│                 │   └─► Auto-select pnpm                     │
│                 │                                            │
│                 └─► Interactive menu                         │
│                     └─► User selects                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Package Manager Download & Setup

```
┌──────────────────────────────────────────────────────────────┐
│              Package Manager Download & Setup                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Cache Check                                              │
│  ─────────────                                               │
│    $CACHE_DIR/vite/package_manager/{pm}/{version}/          │
│         │                                                    │
│         ├─► Exists? → Use cached                             │
│         │                                                    │
│         └─► Missing? → Download                              │
│                  │                                           │
│                  ▼                                           │
│  2. Download with Retry                                      │
│  ──────────────────────                                      │
│    GET https://registry.npmjs.org/{package}/-/              │
│        {package}-{version}.tgz                               │
│         │                                                    │
│         ├─► Success → Extract                                │
│         │                                                    │
│         └─► Failed → Retry with backoff                      │
│             • 1st retry: wait 1s                             │
│             • 2nd retry: wait 2s                             │
│             • 3rd retry: wait 4s                             │
│                  │                                           │
│                  ▼                                           │
│  3. Extract & Setup                                          │
│  ─────────────────                                           │
│    • Extract tgz to temp dir                                 │
│    • Rename package/ to {pm}/                                │
│    • Atomic move to cache dir                                │
│         │                                                    │
│         ▼                                                    │
│  4. Create Shims                                             │
│  ──────────────                                              │
│    For each binary:                                          │
│    • Create Unix shell script (.sh)                          │
│    • Create Windows batch (.cmd)                             │
│    • Create PowerShell script (.ps1)                         │
│         │                                                    │
│         ▼                                                    │
│  Shim Structure:                                             │
│  ──────────────                                              │
│    bin/                                                      │
│    ├── pnpm         → ../pnpm.cjs                            │
│    ├── pnpm.cmd     → ..\pnpm.cjs                            │
│    ├── pnpm.ps1     → ../pnpm.cjs                            │
│    ├── pnpx         → ../pnpx.cjs                            │
│    ├── pnpx.cmd     → ..\pnpx.cjs                            │
│    └── pnpx.ps1     → ../pnpx.cjs                            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Fingerprint Generation

```
┌──────────────────────────────────────────────────────────────┐
│                   Fingerprint Generation                     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Install Fingerprint Components:                             │
│  ───────────────────────────────                             │
│                                                              │
│  1. Configuration Files                                      │
│  ──────────────────────                                      │
│    • package.json content hash                               │
│    • Lock file content hash                                  │
│    • .npmrc/.yarnrc/.pnpmfile (if exists)                    │
│         │                                                    │
│         ▼                                                    │
│  2. Node Modules Structure                                   │
│  ─────────────────────────                                   │
│    Special handling for node_modules:                        │
│    • Ignore file contents                                    │
│    • Record first-level directory names only                 │
│    • Example fingerprint:                                    │
│      node_modules/                                           │
│      ├── @types        (recorded)                            │
│      ├── typescript    (recorded)                            │
│      └── vite          (recorded)                            │
│         │                                                    │
│         ▼                                                    │
│  3. Environment Context                                      │
│  ─────────────────────                                       │
│    • NODE_ENV value                                          │
│    • Package manager version                                 │
│    • Install command arguments                               │
│         │                                                    │
│         ▼                                                    │
│  Fingerprint Hash:                                           │
│  ────────────────                                            │
│    xxHash3({                                                 │
│      config_files + node_modules_list + env_context          │
│    }) → 0xABCDEF123456789                                    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Cache Hit/Miss Behavior

```
┌──────────────────────────────────────────────────────────────┐
│                  Cache Hit/Miss Behavior                     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Auto-execution Mode:                                        │
│  ───────────────────                                         │
│                                                              │
│    Cache Lookup                                              │
│         │                                                    │
│         ├─► Cache Hit                                        │
│         │   • Skip execution                                 │
│         │   • No output replay                               │
│         │   • Continue to next task                          │
│         │                                                    │
│         └─► Cache Miss                                       │
│             • Execute install                                │
│             • Capture outputs                                │
│             • Generate fingerprint                           │
│                                                              │
│  Manual Execution Mode (`vite install`):                     │
│  ────────────────────────────────                            │
│                                                              │
│    Skip Cache                                                │
│         │                                                    │
│         └─► Always Execute                                   │
│             • Run install command                            │
│             • Generate fingerprint                           │
│                                                              │
│  Fingerprint Validation:                                     │
│  ──────────────────────                                      │
│                                                              │
│    Compare Current vs Cached:                                │
│    • package.json changed?        → Cache miss               │
│    • Lock file changed?           → Cache miss               │
│    • node_modules/* changed?      → Cache miss               │
│    • Install args different?      → Cache miss               │
│    • All match?                   → Cache hit                │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

# `vite-plus install`

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

- `packageManager` field in `package.json` , got `pm@version`
- `devEngines.packageManager` field in `package.json` (WIP)
- `pnpm-lock.yaml` or `pnpm-workspace.yaml` file exists ⇒ `pnpm@latest`
- `yarn.lock` file exists ⇒ `yarn@latest`
- `package-lock.json` exists ⇒ `npm@latest`
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

### Install Command Execution Flow

```
vite install
    │
    ├─► Detect Workspace Root
    │   ├─► pnpm-workspace.yaml?  ────► Use as root
    │   ├─► package.json/workspaces? ──► Use as root  
    │   └─► Search upward for package.json
    │       └─► Not found? ────────────► Use current dir
    │
    ├─► Detect Package Manager
    │   ├─► package.json#packageManager? ──► Parse pm@version
    │   ├─► pnpm-lock.yaml exists? ────────► pnpm@latest
    │   ├─► yarn.lock exists? ─────────────► yarn@latest
    │   ├─► package-lock.json exists? ─────► npm@latest
    │   └─► No PM detected?
    │       ├─► CI environment? ───────────► Auto-select pnpm
    │       ├─► Non-TTY? ──────────────────► Auto-select pnpm
    │       └─► Interactive menu ──────────► User selects
    │
    ├─► Install Package Manager (if needed)
    │   ├─► Check shim binary files exists
    │   └─► Not exists?
    │       ├─► Download with retry (exponential backoff)
    │       ├─► Extract tgz archive
    │       ├─► Create shims (sh/cmd/ps1)
    │       └─► Update PATH
    │
    ├─► Execute Install
    │   ├─► Run: {pm} install
    │   ├─► Collect fingerprint
    │   │   ├─► Hash package.json
    │   │   ├─► Hash lock file
    │   │   └─► List node_modules/* (names only)
    │   └─► Save fingerprint to cache
    │
    └─► Post-Install
        ├─► Update package.json (add packageManager if missing)
        └─► Complete
```

### Simplified Component Flow

```
┌─────────┐     ┌──────────┐     ┌───────────┐     ┌─────────┐
│  User   │────►│  CLI     │────►│ Detector  │────►│ Cache   │
└─────────┘     └──────────┘     └───────────┘     └─────────┘
                      │                 │                 │
                      ▼                 ▼                 ▼
                ┌────────────┐     ┌───────────┐    ┌──────────────┐
                │ Downloader │     │ Installer │    │ Fingerprint  │
                └────────────┘     └───────────┘    └──────────────┘
```

### Decision Tree

```
   [Start]
      │
┌─────▼───────┐
│  Find Root? │
└─────┬───────┘
  Yes │ No → Use CWD
      │
┌─────▼─────┐
│  Find PM? │
└─────┬─────┘
  Yes │ No → Select PM
      │      │
      │   ┌──▼──┐
      │   │ CI? │→ Yes → pnpm
      │   └──┬──┘
      │      │ No
      │   ┌──▼──┐
      │   │TTY? │→ No → pnpm
      │   └──┬──┘
      │      │ Yes → Menu
      │      │
┌─────▼──────▼─────┐
│  PM Installed?   │
└─────┬────────────┘
  Yes │ No → Download
      │      │
      │   ┌──▼────────┐
      │   │  Extract  │
      │   │  & Shims  │
      │   └──┬────────┘
      │      │
┌─────▼──────▼─────┐
│   Run Install    │
└─────┬────────────┘
      │
┌─────▼───────┐
│ Fingerprint │
└─────┬───────┘
      │
   [Done]
```

### Key Components

#### 1. Workspace Root Detector

- Searches for monorepo markers (`pnpm-workspace.yaml`, `workspaces` in package.json)
- Falls back to searching for nearest `package.json`
- Uses current directory if no package.json found

#### 2. Package Manager Detector

- Priority order:
  1. `packageManager` field in package.json
  2. Lock file detection (pnpm-lock.yaml, yarn.lock, package-lock.json)
  3. Interactive user selection with pnpm as default

#### 3. Package Manager Installer

- Downloads package manager if not present
- Creates platform-specific shims (sh, cmd, ps1)
- Sets up local installation without global pollution
- Implements retry logic with exponential backoff

#### 4. Fingerprint Collector

- Captures installation state for caching
- Special handling for node_modules (only first-level names)
- Ignores paths outside working directory

#### 5. Auto-execution Handler

- Runs automatically before other vite+ commands
- Skips replay output when cache hit
- Manual execution available with `--force` flag

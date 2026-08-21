# command_env_use

## `vp env use --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp env use [OPTIONS] [REQUESTS]...

Activate Node.js and package-manager versions for this shell session

Arguments:
  [REQUESTS]...  Component selectors or explicit versions to activate

Options:
  --unset                Remove session override (revert to file-based resolution)
  --no-install           Skip auto-installation if version not present
  --silent-if-unchanged  Suppress output if version is already active
  -h, --help             Print help (see a summary with '-h')

Examples:
  vp env use 22.19.0  # Override Node.js for this session
  vp env use pnpm@12  # Override the package manager
  vp env use --unset  # Clear both session overrides

Documentation: https://viteplus.dev/guide/env
```

## `vp env use 20.18.0 --no-install`

should output export command to stdout

```
export VP_NODE_VERSION=20.18.0
Using Node.js <version> (resolved from 20.18.0)
```

## `vp env use --unset`

should output unset command to stdout

```
unset VP_NODE_VERSION
unset VP_PACKAGE_MANAGER
Reverted selected components to project environment resolution
```

## `vp env use d`

should show friendly error for invalid version

**Exit code:** 1

```
error: Invalid Node.js version: "d"

Valid examples:
  vp env use 20          # Latest Node.js 20.x
  vp env use 20.18.0     # Exact version
  vp env use lts         # Latest LTS version
  vp env use latest      # Latest version
```

## `vp env use abc`

should show friendly error for invalid version

**Exit code:** 1

```
error: Invalid Node.js version: "abc"

Valid examples:
  vp env use 20          # Latest Node.js 20.x
  vp env use 20.18.0     # Exact version
  vp env use lts         # Latest LTS version
  vp env use latest      # Latest version
```

## `VP_NODE_VERSION=20.18.0 VP_PACKAGE_MANAGER=npm@10.9.4 vp env use --silent-if-unchanged --no-install`

an unchanged project environment emits no shell mutations

```
```

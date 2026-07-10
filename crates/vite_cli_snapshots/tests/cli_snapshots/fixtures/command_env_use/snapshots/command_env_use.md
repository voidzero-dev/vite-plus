# command_env_use

## `vp env use --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp env use [OPTIONS] [VERSION]

Use a specific Node.js version for this shell session

Arguments:
  [VERSION]  Version to use (e.g., "20", "20.18.0", "lts", "latest"). If omitted, reads from .node-version or package.json

Options:
  --unset                Remove session override (revert to file-based resolution)
  --no-install           Skip auto-installation if version not present
  --silent-if-unchanged  Suppress output if version is already active
  -h, --help             Print help (see a summary with '-h')

Examples:
  vp env use lts        # Override session with latest LTS
  vp env use --unset    # Clear the session override

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
Reverted to file-based Node.js version resolution
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

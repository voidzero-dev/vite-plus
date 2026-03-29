# Environment

`vp env` manages Node.js versions globally and per project.

## Overview

Managed mode is on by default, so `node`, `npm`, and related shims resolve through Vite+ and pick the right Node.js version for the current project.

By default, Vite+ stores its managed runtime and related files in `~/.vite-plus`. If needed, you can override that location with `VP_HOME`.

If you want to keep that behavior, run:

```bash
vp env on
```

This enables managed mode, where the shims always use the Vite+-managed Node.js installation.

If you do not want Vite+ to manage Node.js first, run:

```bash
vp env off
```

This switches to system-first mode, where the shims prefer your system Node.js and only fall back to the Vite+-managed runtime when needed.

## Commands

### Setup

- `vp env setup` creates or updates shims in `VP_HOME/bin`
- `vp env on` enables managed mode so shims always use Vite+-managed Node.js
- `vp env off` enables system-first mode so shims prefer system Node.js first
- `vp env print` prints the shell snippet for the current session

### Manage

- `vp env default` sets or shows the global default Node.js version
- `vp env pin` pins a Node.js version in the current directory
- `vp env unpin` removes `.node-version` from the current directory
- `vp env use` sets a Node.js version for the current shell session
- `vp env install` installs a Node.js version
- `vp env uninstall` removes an installed Node.js version
- `vp env exec` runs a command with a specific Node.js version

### Inspect

- `vp env current` shows the current resolved environment
- `vp env doctor` runs environment diagnostics
- `vp env which` shows which tool path will be used
- `vp env list` shows locally installed Node.js versions
- `vp env list-remote` shows available Node.js versions from the registry

## Project Setup

- Pin a project version with `.node-version`
- Use `vp install`, `vp dev`, and `vp build` normally
- Let Vite+ pick the right runtime for the project

## Examples

```bash
# Setup
vp env setup                  # Create shims for node, npm, npx
vp env on                     # Use Vite+ managed Node.js
vp env print                  # Print shell snippet for this session

# Manage
vp env pin lts                # Pin the project to the latest LTS release
vp env install                # Install the version from .node-version or package.json
vp env default lts            # Set the global default version
vp env use 20                 # Use Node.js 20 for the current shell session
vp env use --unset            # Remove the session override

# Inspect
vp env current                # Show current resolved environment
vp env current --json         # JSON output for automation
vp env which node             # Show which node binary will be used
vp env list-remote --lts      # List only LTS versions

# Execute
vp env exec --node lts npm i  # Execute npm with latest LTS
vp env exec node -v           # Use shim mode with automatic version resolution
```

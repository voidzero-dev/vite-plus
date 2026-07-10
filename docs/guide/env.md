# Environment

`vp env` manages Node.js versions globally and per project.

## Overview

Managed mode is on by default, so `node`, `npm`, and related shims resolve through Vite+ and pick the right Node.js version for the current project.

The project Node.js version is resolved from these sources, in priority order:

1. `.node-version` file (current or parent directories)
2. `devEngines.runtime` in `package.json` (the [devEngines standard](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devengines))
3. `engines.node` in `package.json`
4. The global default (`vp env default`), then the latest LTS

`devEngines.runtime` ranks above `engines.node` because it declares the development-environment requirement, while `engines.node` is a consumer-facing support range. `vp env doctor` warns when declared sources conflict.

When a project declares `packageManager` (or `devEngines.packageManager`) in `package.json`, matching package-manager shims also use that package-manager version. For example, `packageManager: "npm@10.9.4"` makes both `npm` and `npx` run through npm 10.9.4. Alias pairs follow the installed package-manager shims: `npm`/`npx`, `pnpm`/`pnpx`, `yarn`/`yarnpkg`, and `bun`/`bunx`. Vite+ does not translate mismatched commands, so a project pinned to `pnpm` still lets `npm` fall back to the npm that comes with the resolved Node.js runtime.

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

- `vp env setup` creates or updates shims in `VP_HOME/bin` (and writes the per-shell setup scripts under `VP_HOME`)
- `vp env on` enables managed mode so shims always use Vite+-managed Node.js
- `vp env off` enables system-first mode so shims prefer system Node.js first
- `vp env print` prints the shell snippet for the current session

PowerShell needs to dot-source the generated setup script in the current shell before `vp env use` can affect only that shell session:

```powershell
. "$env:USERPROFILE\.vite-plus\env.ps1"
```

Add that line to the end of your PowerShell `$PROFILE` to apply it automatically in new shells. It does not require elevated privileges.

Create the profile file if it does not already exist:

```powershell
if (-not (Test-Path $PROFILE)) { New-Item $PROFILE -Force }
```

Open the profile file for editing:

```powershell
Invoke-Item $PROFILE
```

Windows Command Prompt (`cmd.exe`) cannot define the wrapper function needed for `vp env use` to update the current shell session. Use the generated `vp-use.cmd` command instead:

```batch
vp-use 20
node --version
vp-use --unset
```

Only `vp env use` needs this alternate command. Other `vp env` commands work normally in Command Prompt. `vp env setup` creates `vp-use.cmd` under `VP_HOME/bin` on Windows.

In CI, `vp env use` can still run without shell initialization. It writes a temporary session file under `VP_HOME` so later shim calls in the same job can resolve the selected Node.js version.

### Manage

- `vp env default` sets or shows the global default Node.js version
- `vp env pin` pins a Node.js version in the current directory: an existing `.node-version` keeps being updated; otherwise the pin is written to `package.json#devEngines.runtime`; `.node-version` is only created when the directory has no `package.json`. Use `--target node-version` or `--target dev-engines` to choose explicitly. An existing `engines.node` is never modified.
- `vp env unpin` removes the pin from the same source `vp env pin` would write
- `vp env use` sets a Node.js version for the current shell session
- `vp env install` installs a Node.js version
- `vp env uninstall` removes an installed Node.js version
- `vp env clean` removes unused managed Node.js runtimes, all downloaded package managers, and the Corepack cache.
- `vp env exec` runs a command with a specific Node.js version
- `vp node` runs a Node.js script — shorthand for `vp env exec node`

### Inspect

- `vp env current` shows the current resolved environment
- `vp env doctor` runs environment diagnostics
- `vp env which` shows which tool path will be used
- `vp env list` shows locally installed Node.js versions
- `vp env list-remote` shows available Node.js versions from the registry

## Project Setup

- Pin a project version with `vp env pin`
- Use `vp install`, `vp dev`, and `vp build` normally
- Let Vite+ pick the right runtime for the project

## Examples

```bash
# Setup
vp env setup                  # Create shims for node, npm, npx, corepack
vp env on                     # Use Vite+ managed Node.js
vp env print                  # Print shell snippet for this session

# Manage
vp env pin lts                # Pin the project to the latest LTS release
vp env install                # Install the version from .node-version or package.json
vp env default lts            # Set the global default version
vp env use 20                 # Use Node.js 20 for the current shell session
vp env use --unset            # Remove the session override
vp env clean                  # Remove unused managed caches

# Inspect
vp env current                # Show current resolved environment
vp env current --json         # JSON output for automation
vp env which node             # Show which node binary will be used
vp env which npx              # Show pinned package-manager alias when packageManager matches
vp env list-remote --lts      # List only LTS versions

# Execute
vp env exec --node lts npm i  # Execute npm with latest LTS
vp env exec node -v           # Use shim mode with automatic version resolution
vp node script.js             # Shorthand: run a Node.js script with the resolved version
vp node -e "console.log(1+1)" # Shorthand: forward any node flag or argument
```

## Corepack

Vite+ creates a `corepack` shim by default, so corepack works without a system Node.js installation:

- On Node.js 24 and earlier, the shim runs the corepack bundled with the resolved Node.js version.
- On Node.js 25 and later, where corepack is no longer bundled, Vite+ installs corepack as a managed global package on first use. Only the `corepack` binary is linked; run `vp install -g corepack` yourself if you also want the package's pnpm/yarn launchers exposed directly.
- If you install corepack explicitly with `vp install -g corepack`, that installation is always preferred.

`corepack enable` normally creates `pnpm`/`yarn` launchers next to the corepack binary, which under Vite+ would not be on `PATH`. The shim fixes this by defaulting `--install-directory` to `VP_HOME/bin`, so after `corepack enable` the launchers are available everywhere and still resolve the project's Node.js and package-manager versions:

```bash
corepack enable               # pnpm and yarn now resolve via corepack
corepack disable              # Remove the pnpm/yarn launchers again
```

The launchers reference the corepack copy that created them. If that copy is later removed (for example by uninstalling the Node.js version it shipped with), rerun `corepack enable` to recreate them.

Shims owned by Vite+ (`npm`, `npx`, and binaries installed with `vp install -g`) are protected: if corepack removes or replaces them, Vite+ restores them and prints a warning.

## Custom Node.js Mirror

By default, Vite+ downloads Node.js from `https://nodejs.org/dist`. If you're behind a corporate proxy or need to use an internal mirror (e.g., Artifactory), set the `VP_NODE_DIST_MIRROR` environment variable:

```bash
# Install a specific version from your custom mirror
VP_NODE_DIST_MIRROR=https://my-mirror.example.com/nodejs/dist vp env install 22

# Set the global default version using a custom mirror
VP_NODE_DIST_MIRROR=https://my-mirror.example.com/nodejs/dist vp env default lts

# Set it permanently in your shell profile (.bashrc, .zshrc, etc.)
echo 'export VP_NODE_DIST_MIRROR=https://my-mirror.example.com/nodejs/dist' >> ~/.zshrc
```

## Node.js Signature Verification

When installing Node.js from the official `nodejs.org` distribution, Vite+ downloads the PGP-signed `SHASUMS256.txt.asc` and verifies it against the bundled Node.js release keys before trusting any checksum. This protects against a tampered `SHASUMS256.txt` paired with a matching malicious archive. The SHA-256 checksum of the downloaded archive is always verified afterward.

Custom mirrors (`VP_NODE_DIST_MIRROR`) that publish only the plain `SHASUMS256.txt` fall back to checksum-only verification. A mirror that does publish a `.asc` still has its signature verified, and an invalid signature is a hard error.

If a future keyring or certificate issue blocks downloads, set `VP_NODE_SKIP_SIGNATURE_VERIFY` to temporarily bypass PGP verification. The SHA-256 checksum is still verified, and Vite+ prints a warning when the signature check is skipped:

```bash
VP_NODE_SKIP_SIGNATURE_VERIFY=1 vp env install 22
```

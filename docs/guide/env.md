# Environment

`vp env` manages the complete JavaScript environment: one Node.js runtime and one selected package manager. npm, pnpm, Yarn, and Bun are peer package-manager families.

## Overview

Managed mode is on by default, so Node.js and configured package-manager shims resolve through Vite+ and pick the right versions for the current project. Fresh installers record managed mode for npm, pnpm, Yarn, and Bun after the user enables environment management.

When an upgrade adds a package-manager shim that has no recorded mode, its first interactive invocation asks what to do only when the corresponding system binary is already on PATH. The current family defaults to managed mode; choosing a system tool or applying a choice to every family remains explicit. Non-interactive invocations use managed mode without recording a choice.

Most commands operate on both components when no selector is given. Add `node`, `pm`, `npm`, `pnpm`, `yarn`, or `bun` to narrow the command. `pm` means all four families for listing and cleanup, but the single selected package manager for project operations.

Unqualified versions remain Node.js versions for compatibility:

```bash
vp env pin 22.0.0               # Node.js only
vp env pin pnpm@10.18.0         # pnpm only
vp env pin node@24 pnpm@12      # Both components
vp env pin 22.0.0 pnpm@10.18.0  # Also both components
```

Vite+ checks the current directory first, then walks up through its parents. The nearest directory
with a supported declaration wins. Within each directory, sources are checked in this order:

1. `.node-version` file
2. `devEngines.runtime` in `package.json` (the [devEngines standard](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devengines))
3. `engines.node` in `package.json`
4. `.nvmrc` file

If no directory declares a version, Vite+ uses the global default (`vp env default`) and then the
latest LTS.

`devEngines.runtime` ranks above `engines.node` because it declares the development-environment requirement, while `engines.node` is a consumer-facing support range. `vp env doctor` warns when declared sources conflict.

Package-manager selection uses this priority:

1. Explicit command override
2. `VP_PACKAGE_MANAGER` or the shell-session override
3. Top-level `packageManager`
4. `devEngines.packageManager`
5. Lockfile or manager-specific configuration
6. The named package manager's global default version
7. The named shim's latest release

A selected manager controls only its named shims. For example, pnpm controls `pnpm` and `pnpx`; invoking `npm` still resolves npm independently. Alias pairs are `npm`/`npx`, `pnpm`/`pnpx`, `yarn`/`yarnpkg`, and `bun`/`bunx`. Without a matching project selection, a named shim uses its configured default version and otherwise uses the latest release without prompting. The resolved version is cached for one hour and an expired cache remains available when the registry cannot be reached. The directly invoked npm shim keeps its Node-bundled fallback, while an explicit `vp env ... npm` family scope uses standalone npm's latest release.

A fresh install uses the split platform layout by default. On Unix, Vite+
stores managed runtimes and related files in `~/.local/share/vite-plus`. It
stores executables in the Vite+-owned `~/.local/share/vite-plus/bin` directory.
On Windows, Vite+ uses `%LOCALAPPDATA%\vite-plus\data` for data and
`%LOCALAPPDATA%\vite-plus\bin` for executables. Vite+ does not move an existing
`~/.vite-plus` install. `VP_HOME` puts all categories under one custom root.

If you want to keep that behavior, run:

```bash
vp env on
```

This enables managed mode for both components. Their modes can also be changed independently, including one package-manager family:

```bash
vp env on node
vp env off pm
vp env off pnpm
vp env on bun
```

If you do not want Vite+ to manage Node.js first, run:

```bash
vp env off
```

This switches both components to system-first mode. Vite+ prefers system tools and falls back to managed installations. Mixed configurations compose: a system package-manager launcher receives the Node.js selected by the Node mode.

Using `pm` records the selected mode for all currently supported package managers and replaces their individual choices. An unscoped `on` or `off` does the same while also changing Node.js. A family without a recorded mode remains undecided until its shim is first used or an `on` / `off` command configures it.

## Commands

### Setup

- `vp env setup` creates or updates the `node`, `npm`, `npx`, `pnpm`, `pnpx`, `yarn`, `yarnpkg`, `bun`, `bunx`, `vpx`, and `vpr` shims in the resolved bin directory. It writes shell setup scripts in the config directory.
- `vp env on` / `vp env off` changes both modes; append `node`, `pm`, `npm`, `pnpm`, `yarn`, or `bun` to narrow the change
- `vp env print` prints PATH setup for both components; append a selector to print one

PowerShell needs to dot-source the generated setup script in the current shell before `vp env use` can affect only that shell session:

```powershell
. "$env:APPDATA\vite-plus\env.ps1"
```

If an older Vite+ install uses `%USERPROFILE%\.vite-plus`, source the `env.ps1`
file in that directory instead.

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

Only `vp env use` needs this alternate command. Other `vp env` commands work normally in Command Prompt. `vp env setup` creates `vp-use.cmd` in the bin directory on Windows.

In CI, `vp env use` can run without shell initialization. It writes a temporary
Node.js or package-manager session file in the resolved state directory. Later
shim calls in the same job use these files to resolve the same environment.

### Manage

- `vp env default` shows the global Node.js default and each configured package-manager version. Bare versions set Node.js; qualified specs such as `pnpm@10.18.0` set that package manager's shim default without replacing the defaults for Bun, Yarn, or npm. `--unset` clears all defaults unless scoped.
- `vp env pin` shows or writes project pins. Existing `.node-version` and top-level `packageManager` fields keep being updated for compatibility; otherwise Vite+ writes the matching `devEngines` entry. Use `--target node-version`, `--target dev-engines`, or `--target package-manager` to choose explicitly.
- `vp env unpin` removes both effective pins by default; append a selector to remove one. Lower-priority declarations are not deleted.
- `vp env use` activates the complete project environment. Explicit specs override selected components; `--unset` clears both unless scoped.
- `vp env install` installs the complete resolved environment, a selected component, or explicit specs.
- `vp env uninstall` removes explicit exact Node.js or qualified package-manager versions.
- `vp env clean` removes unused installs. Use `clean node`, `clean pm`, or a concrete manager. Current and configured-default versions are preserved.
- `vp env exec` runs a command in the resolved environment. Use `--node` and `--package-manager`; `--npm` is an alias for `--package-manager npm@…`.
- `vp node` uses the resolved Node.js runtime and exposes the selected package-manager path to child processes.

### Inspect

- `vp env current` shows the current resolved environment
- `vp env doctor` runs environment diagnostics
- `vp env which` shows which tool path will be used
- `vp env list` shows separate Node.js, npm, pnpm, Yarn, and Bun sections; selectors narrow output
- `vp env list-remote` fetches Node.js and all four PM registries concurrently; selectors narrow network work. `--lts` implicitly selects Node.js.

## Project Setup

- Pin a project version with `vp env pin`
- Use `vp install`, `vp dev`, and `vp build` normally
- Let Vite+ pick the right runtime for the project

## Examples

```bash
# Setup
vp env setup                  # Create Node.js and package-manager shims
vp env on                     # Manage Node.js and package managers
vp env off pm                 # Prefer system package managers only
vp env off pnpm               # Prefer system pnpm only
vp env print                  # Print PATH setup for both components

# Manage
vp env pin lts pnpm@10        # Pin both project components to exact versions
vp env install                # Install the complete resolved environment
vp env default node@24        # Set the global Node.js default
vp env default pnpm@10        # Set pnpm's global default version
vp env use 20 pnpm@10         # Override both components for this shell
vp env use --unset pm         # Remove only the PM session override
vp env clean                  # Remove unused managed Node.js and package manager versions

# Inspect
vp env current                # Show current resolved environment
vp env current --json         # JSON output for automation
vp env which node             # Show which node binary will be used
vp env which npx              # Show pinned package-manager alias when packageManager matches
vp env list                   # Show every locally installed component
vp env list node              # Show only Node.js installations
vp env list-remote --lts      # List only Node.js LTS versions

# Execute
vp env exec --node lts --package-manager pnpm@10 pnpm install
vp env exec node -v           # Use shim mode with automatic version resolution
vp node script.js             # Shorthand: run a Node.js script with the resolved version
vp node -e "console.log(1+1)" # Shorthand: forward any node flag or argument
```

## JSON output

The JSON output for `current`, `list`, and `list-remote` is organized by component. `current --json` returns sibling `node` and `package_manager` objects:

```json
{
  "node": {
    "version": "22.0.0",
    "source": "devEngines.runtime",
    "source_path": "/project/package.json",
    "project_root": "/project",
    "bin_path": "/home/.vite-plus/js_runtime/node/22.0.0/bin/node",
    "installed": true,
    "mode": "managed"
  },
  "package_manager": {
    "name": "pnpm",
    "version": "10.18.0",
    "source": "packageManager",
    "source_path": "/project/package.json",
    "project_root": "/project",
    "bin_paths": {
      "pnpm": "/home/.vite-plus/package_manager/pnpm/10.18.0/pnpm/bin/pnpm",
      "pnpx": "/home/.vite-plus/package_manager/pnpm/10.18.0/pnpm/bin/pnpx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```

`list --json` and `list-remote --json` group the component arrays:

```json
{
  "node": [],
  "package_managers": {
    "npm": [],
    "pnpm": [],
    "yarn": [],
    "bun": []
  }
}
```

Selectors omit unselected top-level fields or PM families. Registry listing is all-or-error: Vite+ prints no partial human or JSON result when any selected registry request fails.

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

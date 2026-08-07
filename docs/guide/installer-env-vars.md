# Installer Environment Variables

The Vite+ installers (`vp-setup.exe`, `install.ps1`, and `install.sh`) and the installed `vp` CLI read the environment variables on this page.

## Installation Variables

These variables control the installer scripts and the standalone Windows installer (`vp-setup.exe`).

### `VP_VERSION`

- **Purpose**: Version to install
- **Default**: `latest`
- **CLI equivalent**: `--version`
- **Example**:

  ```bash
  # Unix
  curl -fsSL https://vite.plus | VP_VERSION=1.2.3 bash
  ```

  ```powershell
  # PowerShell
  $env:VP_VERSION = "1.2.3"; irm https://vite.plus/ps1 | iex
  ```

### `VP_HOME` (deprecated)

- **Purpose**: Legacy override that selects the legacy monolithic layout, rooted at the given directory
- **Default**: None — fresh installs use the [split layout](#directory-layout-and-xdg-variables); `~/.vite-plus` is used only when it already exists (grandfathered installs) or when `VP_HOME`/`--install-dir` is set
- **CLI equivalent**: `--install-dir`
- **Details**: Deprecated, but still honored by the installed `vp` CLI as the highest-priority layout rule (everything lives under this one root), so older env scripts and custom-location installs that export it keep working. The installers (`install.sh`, `install.ps1`, `vp-setup.exe`) also accept it as the install dir, and the installers and generated env scripts no longer set it for new installs. Prefer `VP_*_DIR` / `XDG_*` variables. See [Directory Layout and XDG Variables](#directory-layout-and-xdg-variables).
- **Example**:

  ```bash
  # Unix
  curl -fsSL https://vite.plus | VP_HOME=/opt/vite-plus bash
  ```

  ```powershell
  # PowerShell
  $env:VP_HOME = "D:\vite-plus"; irm https://vite.plus/ps1 | iex
  ```

### `NPM_CONFIG_REGISTRY`

- **Purpose**: Custom npm registry URL
- **Default**: `https://registry.npmjs.org`
- **CLI equivalent**: `--registry`
- **Example**:
  ```bash
  curl -fsSL https://vite.plus | NPM_CONFIG_REGISTRY=https://registry.npmmirror.com bash
  ```

### `VP_NODE_MANAGER`

- **Purpose**: Control Node.js version manager setup during installation
- **Values**: `yes` or `no`
- **Default**: Auto-detected
- **CLI equivalent**: `--no-node-manager` (inverted)
- **Example**:
  ```bash
  # Skip Node.js manager setup in CI
  curl -fsSL https://vite.plus | VP_NODE_MANAGER=no bash
  ```

### `VP_PR_VERSION`

- **Purpose**: Install a preview build from a pull request or commit SHA
- **Values**: PR number or commit SHA
- **Default**: None
- **Details**: [Global `vp` Preview](/guide/upgrade#global-vp-preview)

### Development variables

When developing Vite+ itself, `VP_LOCAL_TGZ` (path to a local `vite-plus.tgz`) and `VP_LOCAL_BINARY` (path to a local `vp` binary) feed the installer a local build. The installers also set `VP_INSTALL_STOP` themselves; do not set it manually.

## Runtime Variables

These variables configure the installed Vite+ CLI.

### `VP_BIN_DIR`

- **Purpose**: Directory for executables and shims (`node`, `npm`, `npx`, `corepack`, `vpx`, `vpr`, the `vp` wrapper)
- **Default**: `XDG_BIN_HOME` if set, then `XDG_DATA_HOME/../bin`, otherwise `~/.local/bin` (Unix) or `%LOCALAPPDATA%\vite-plus\bin` (Windows)
- **Details**: Only applies in the split layout; ignored when the legacy monolithic layout is selected. See [Directory Layout and XDG Variables](#directory-layout-and-xdg-variables).

### `VP_DATA_DIR`

- **Purpose**: Payload data directory (CLI versions, managed Node.js runtimes, package managers, global packages)
- **Default**: `XDG_DATA_HOME/vite-plus` if set, otherwise `~/.local/share/vite-plus` (Unix) or `%LOCALAPPDATA%\vite-plus\data` (Windows)
- **Details**: Only applies in the split layout; ignored when the legacy monolithic layout is selected. See [Directory Layout and XDG Variables](#directory-layout-and-xdg-variables).

### `VP_CACHE_DIR`

- **Purpose**: Disposable cache directory
- **Default**: `XDG_CACHE_HOME/vite-plus` if set, otherwise `~/.cache/vite-plus` (Unix) or `%LOCALAPPDATA%\vite-plus\cache` (Windows)
- **Details**: Only applies in the split layout; ignored when the legacy monolithic layout is selected. See [Directory Layout and XDG Variables](#directory-layout-and-xdg-variables).

### `VP_NODE_DIST_MIRROR`

- **Purpose**: Node.js distribution mirror URL
- **Default**: `https://nodejs.org/dist`
- **Details**: [Custom Node.js Mirror](/guide/env#custom-node-js-mirror)

### `VP_NODE_VERSION`

- **Purpose**: Override Node.js version
- **Default**: None (auto-detected)
- **Example**:
  ```bash
  # Run a command with a specific Node.js version
  VP_NODE_VERSION=22 vp env exec node -v
  ```

### `VP_NODE_SKIP_SIGNATURE_VERIFY`

- **Purpose**: Skip PGP signature verification of Node.js downloads
- **Values**: Any non-empty value
- **Default**: None (verification enabled)
- **Details**: [Node.js Signature Verification](/guide/env#node-js-signature-verification)

### `VP_SHELL`

- **Purpose**: Specify the current shell
- **Default**: Auto-detected
- **Example**:
  ```bash
  VP_SHELL=bash vp env print
  ```

### `VP_BYPASS`

- **Purpose**: Bypass the Vite+ shim and use the system tool
- **Values**: `PATH`-style list of directories to bypass
- **Default**: None
- **Example**:
  ```bash
  VP_BYPASS=/usr/local/bin node -v
  ```

### Internal variables

Vite+ sets additional `VP_*` variables during shim dispatch and shell integration (recursion guards, active-version records, wrapper flags); do not set them manually.

## TLS/CA Configuration

### `SSL_CERT_FILE` / `NODE_EXTRA_CA_CERTS`

- **Purpose**: Path to PEM bundle of extra CA certificates (`NODE_EXTRA_CA_CERTS` is the Node.js convention)
- **Default**: System trust store
- **Example**:
  ```bash
  export SSL_CERT_FILE=/path/to/custom-ca.pem
  ```

### `VP_INSECURE_TLS`

- **Purpose**: Disable HTTPS certificate verification
- **Values**: Any non-empty value (`1`, `true`, `yes`)
- **Default**: None (verification enabled)
- **Warning**: Diagnostic escape hatch only; do not use in production
- **Example**:
  ```bash
  VP_INSECURE_TLS=1 vp env install 22
  ```

## Logging and Debugging

### `VP_LOG`

- **Purpose**: Log filter string for `tracing_subscriber`
- **Default**: None
- **Example**:
  ```bash
  VP_LOG=debug vp dev
  VP_LOG=vt=trace vp build
  ```

### `VP_DEBUG_SHIM`

- **Purpose**: Enable debug output for shim dispatch
- **Values**: Any non-empty value
- **Default**: None
- **Example**:
  ```bash
  VP_DEBUG_SHIM=1 node -v
  ```

## Standard Environment Variables

Vite+ also respects these standard environment variables:

### `CI`

- **Purpose**: Indicates running in CI environment
- **Effect**: Enables silent mode (`--yes`) for installers

### `NO_COLOR`

- **Purpose**: Disable colored output
- **Effect**: Disables ANSI color codes

### `HOME` / `USERPROFILE`

- **Purpose**: User home directory
- **Effect**: Base for the legacy `~/.vite-plus` root and the Unix platform defaults (`~/.local/bin`, `~/.config`, ...)

### `XDG_BIN_HOME` / `XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_STATE_HOME` / `XDG_CACHE_HOME`

- **Purpose**: XDG base directories honored when resolving the split layout
- **Details**: Read directly from the process environment during directory resolution. See [Directory Layout and XDG Variables](#directory-layout-and-xdg-variables).

## Directory Layout and XDG Variables

The installed CLI (`VpDirs` in `vp_shared`) resolves where its files live by walking an ordered chain; the first match wins. There is **no** executable self-location or `PATH`-based install discovery — only env overrides, on-disk grandfathering, and platform defaults.

1. **`VP_HOME` is set** (deprecated) — the legacy monolithic layout rooted at its value; every category lives under this one root.
2. **`~/.vite-plus` exists** — the legacy monolithic layout, grandfathered: existing installs keep working untouched and nothing is moved. (A process-cwd `./.vite-plus` is also recognized for project-local / test fixtures.)
3. **Otherwise (fresh / split installs)** — each category resolves independently through its own override → XDG (Unix) → platform-default chain:

| Category              | Contents                                                                                                     | Resolution (first match wins)                          | Unix default               | Windows default                  |
| --------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------ | -------------------------- | -------------------------------- |
| Executables and shims | `node`, `npm`, `npx`, `corepack`, `vpx`, `vpr`, the `vp` wrapper                                             | `VP_BIN_DIR` → `XDG_BIN_HOME` → `XDG_DATA_HOME/../bin` | `~/.local/bin`             | `%LOCALAPPDATA%\vite-plus\bin`   |
| Configuration         | `config.json`, shell env scripts                                                                             | `XDG_CONFIG_HOME/vite-plus`                            | `~/.config/vite-plus`      | `%APPDATA%\vite-plus`            |
| Data                  | CLI versions, managed Node.js runtimes, package managers, global packages, per-binary `bins/*.json` metadata | `VP_DATA_DIR` → `XDG_DATA_HOME/vite-plus`              | `~/.local/share/vite-plus` | `%LOCALAPPDATA%\vite-plus\data`  |
| State                 | Session and upgrade-check files                                                                              | `XDG_STATE_HOME/vite-plus`                             | `~/.local/state/vite-plus` | `%LOCALAPPDATA%\vite-plus\state` |
| Cache                 | Disposable caches                                                                                            | `VP_CACHE_DIR` → `XDG_CACHE_HOME/vite-plus`            | `~/.cache/vite-plus`       | `%LOCALAPPDATA%\vite-plus\cache` |

Notes:

- Relative values in the `VP_*_DIR` and `XDG_*` variables are ignored, per the XDG Base Directory specification.
- `VP_BIN_DIR`, `VP_DATA_DIR`, and `VP_CACHE_DIR` only apply in the split layout; the legacy layout (rules 1–2) is all-or-nothing.
- `VP_HOME` is deprecated: still honored as rule 1 for backward compatibility, but no longer set by the installers or the generated env scripts. Prefer `VP_*_DIR` / `XDG_*` variables.
- Custom split roots require the corresponding `VP_*_DIR` (and/or `XDG_*`) variables to remain set in the environment for later CLI invocations — installers pin them only for the install session itself. Generated env scripts put the **bin** directory on `PATH`; they do not re-export category overrides.
- The installers (`install.sh`, `install.ps1`, `vp-setup.exe`) use the same precedence: `VP_HOME` / `--install-dir` → existing `~/.vite-plus` → split defaults (`VP_*_DIR` / `XDG_*` / platform). Fresh installs land on the split layout; an existing `~/.vite-plus` keeps the legacy layout.

## Precedence

1. CLI flags (highest priority)
2. Environment variables
3. Default values (lowest priority)

For example, `VP_VERSION=1.0.0 vp-setup.exe --version 2.0.0` installs version 2.0.0.

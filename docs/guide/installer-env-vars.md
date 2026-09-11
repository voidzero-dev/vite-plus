# Installer Environment Variables

The Vite+ installers (`vp-setup.exe`, `install.ps1`, and `install.sh`) and the installed `vp` CLI read the environment variables on this page.

## Installation Variables

These variables control the installer scripts and the standalone Windows installer (`vp-setup.exe`).

### `VP_VERSION`

- **Purpose**: Version to install
- **Default**: `latest`
- **CLI equivalent**: `--version`
- **Note**: Vite+ 0.2.x and earlier do not support the split directory layout.
  The installer always puts these releases in the monolithic root (`VP_HOME` or
  `~/.vite-plus`). This rule also applies to a fresh machine. The installer
  checks the downloaded binary and prints a notice.
- **Example**:

  ```bash
  # Unix
  curl -fsSL https://vite.plus | VP_VERSION=1.2.3 bash
  ```

  ```powershell
  # PowerShell
  $env:VP_VERSION = "1.2.3"; irm https://vite.plus/ps1 | iex
  ```

### `VP_HOME`

- **Purpose**: Optional pin for the single-root layout. Set it to an absolute
  path. Vite+ then puts bin, data, cache, config, and state under that directory.
  The installed CLI reads the same variable. See [Environment](/guide/env).
- **Default**: unset. Vite+ reuses an existing install in `~/.vite-plus` on
  Unix or `%USERPROFILE%\.vite-plus` on Windows. The directory must contain a
  `current` link. Otherwise, a fresh install uses the split platform layout. On
  Unix, it uses `~/.local/share/vite-plus` and its Vite+-owned `bin`
  subdirectory. On Windows, it uses `%LOCALAPPDATA%\vite-plus\data` and
  `%LOCALAPPDATA%\vite-plus\bin`.
- **Example**:

  ```bash
  # Unix
  curl -fsSL https://vite.plus | VP_HOME=/opt/vite-plus bash
  ```

  ```powershell
  # PowerShell
  $env:VP_HOME = "D:\vite-plus"; irm https://vite.plus/ps1 | iex
  ```

### `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`

- **Purpose**: Internal absolute directory overrides for integrations that
  must pin a split install. Set all three variables together. The installer
  rejects an incomplete group. Vite+ ignores the group when `VP_HOME` is set
  or when it reuses an existing `~/.vite-plus` install.
- **Default**: unset (XDG / platform defaults)
- **Persistence**: The generated environment file does not export these
  variables. An integration that uses them must provide the complete group to
  each Vite+ process.
- **Example**:

  ```bash
  export VP_DATA_DIR=$HOME/vite-plus-data
  export VP_BIN_DIR=$VP_DATA_DIR/bin
  export VP_CACHE_DIR=$HOME/.cache/vite-plus
  curl -fsSL https://vite.plus | bash
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

Use `VP_LOCAL_TGZ` and `VP_LOCAL_BINARY` when you develop Vite+ itself.
`VP_LOCAL_TGZ` specifies a local `vite-plus.tgz` file. `VP_LOCAL_BINARY`
specifies a local `vp` binary. The installers use these files for the local
build. They use `VP_DUMP_DIRS=1` to get the layout mode and all five `EnvConfig`
category roots from the selected binary. They do not resolve the directory
variables. The installers set `VP_INSTALL_STOP`; do not set it manually.

## Runtime Variables

These variables configure the installed Vite+ CLI. `VP_HOME` (above) also applies at runtime.

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

### `VP_PACKAGE_MANAGER`

- **Purpose**: Override the selected package manager and version
- **Default**: None (resolved from the project or global default)
- **Format**: `npm|pnpm|yarn|bun@<version>`
- **Example**:
  ```bash
  VP_PACKAGE_MANAGER=pnpm@10.18.0 vp install
  ```

### `VP_NODE_SKIP_SIGNATURE_VERIFY`

- **Purpose**: Skip PGP signature verification of Node.js downloads
- **Values**: Any non-empty value
- **Default**: None (verification enabled)
- **Details**: [Node.js Signature Verification](/guide/env#node-js-signature-verification)

### `VP_DOWNLOAD_TIMEOUT`

- **Purpose**: Per-request timeout, in seconds, for large downloads such as Node.js runtimes and package-manager tarballs
- **Values**: Positive integer, at most `86400` (24 hours); invalid values are ignored with a warning
- **Default**: `600` (10 minutes)
- **Example**:
  ```bash
  # Allow up to 30 minutes per download on a slow connection
  VP_DOWNLOAD_TIMEOUT=1800 vp env install 22
  ```

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
- **Installer behavior**: When `CI=true`, `install.sh` hides shell file errors.
  Set `VP_LOG=trace` to show these errors.
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
- **Effect**: Base for the existing-install probe (`~/.vite-plus`) and for split platform defaults

## Precedence

1. CLI flags (highest priority)
2. Environment variables
3. Default values (lowest priority)

For example, `VP_VERSION=1.0.0 vp-setup.exe --version 2.0.0` installs version 2.0.0.

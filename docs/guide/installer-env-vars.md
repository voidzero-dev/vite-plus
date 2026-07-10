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

### `VP_HOME`

- **Purpose**: Installation directory; the installed CLI reads the same variable as the Vite+ home directory (see [Environment](/guide/env))
- **Default**: `~/.vite-plus` (Unix) or `%USERPROFILE%\.vite-plus` (Windows)
- **CLI equivalent**: `--install-dir`
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

These variables configure the installed Vite+ CLI. `VP_HOME` (above) also applies at runtime.

### `VP_NODE_DIST_MIRROR`

- **Purpose**: Node.js distribution mirror URL
- **Default**: `https://nodejs.org/dist`
- **Details**: [Custom Node.js Mirror](/guide/env#custom-nodejs-mirror)

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
- **Details**: [Node.js Signature Verification](/guide/env#nodejs-signature-verification)

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

### `VITE_LOG`

- **Purpose**: Log filter string for `tracing_subscriber`
- **Default**: None
- **Example**:
  ```bash
  VITE_LOG=debug vp dev
  VITE_LOG=vite_task=trace vp build
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
- **Effect**: Base for the default `~/.vite-plus` path

## Precedence

1. CLI flags (highest priority)
2. Environment variables
3. Default values (lowest priority)

For example, `VP_VERSION=1.0.0 vp-setup.exe --version 2.0.0` installs version 2.0.0.

# Installer Environment Variables

This page documents all environment variables recognized by the Vite+ installers (`vp-setup.exe`, `install.ps1`, and `install.sh`).

::: tip
The set of supported variables may change between releases. Always check the [release notes](https://github.com/voidzero-dev/vite-plus/releases) for the version you are using.
:::

## Installation Variables

These variables control the behavior of the Vite+ installer scripts and the standalone Windows installer (`vp-setup.exe`).

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

- **Purpose**: Installation directory
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
- **Default**: Auto-detected based on environment
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
- **Example**:
  ```bash
  # Install preview build for PR #1569
  curl -fsSL https://vite.plus | VP_PR_VERSION=1569 bash
  ```

### `VP_LOCAL_TGZ`

- **Purpose**: Path to local `vite-plus.tgz` for development/testing
- **Default**: None
- **Example**:
  ```bash
  curl -fsSL https://vite.plus | VP_LOCAL_TGZ=./vite-plus-0.0.0.tgz bash
  ```

### `VP_LOCAL_BINARY`

- **Purpose**: Path to local `vp` binary for development
- **Default**: None
- **Note**: Set by `install-global-cli.ts` for local development

### Internal (do not set manually)

These variables are used internally by the installers and should not be set manually:

- `VP_INSTALL_STOP` — Signal to stop installation (used internally by `install.ps1`)

## Runtime Variables

These variables affect the behavior of the installed Vite+ CLI.

### `VP_HOME`

- **Purpose**: Override the Vite+ home directory
- **Default**: `~/.vite-plus`
- **Example**:
  ```bash
  export VP_HOME=/opt/vite-plus
  ```

### `VP_NODE_DIST_MIRROR`

- **Purpose**: Node.js distribution mirror URL
- **Default**: `https://nodejs.org/dist`
- **Example**:
  ```bash
  export VP_NODE_DIST_MIRROR=https://npmmirror.com/mirrors/node
  ```

### `VP_NODE_VERSION`

- **Purpose**: Override Node.js version
- **Default**: None (auto-detected)
- **Example**:

  ```bash
  # Run a command with a specific Node.js version
  VP_NODE_VERSION=22 vp env exec node -v
  ```

  ```cmd
  :: CMD
  set VP_NODE_VERSION=22 && vp env exec node -v
  ```

### `VP_NODE_SKIP_SIGNATURE_VERIFY`

- **Purpose**: Skip PGP signature verification of Node.js downloads
- **Values**: Any non-empty value
- **Default**: None (verification enabled)
- **Example**:
  ```bash
  VP_NODE_SKIP_SIGNATURE_VERIFY=1 vp env install 22
  ```

### `VP_DEBUG_SHIM`

- **Purpose**: Enable debug output for shim dispatch
- **Values**: Any non-empty value
- **Default**: None
- **Example**:
  ```bash
  VP_DEBUG_SHIM=1 node -v
  ```

### `VP_ENV_USE_EVAL_ENABLE`

- **Purpose**: Enable eval mode for `vp env use`
- **Values**: Any non-empty value
- **Default**: None
- **Example**:
  ```bash
  VP_ENV_USE_EVAL_ENABLE=1 eval "$(vp env use 20)"
  ```

### `VP_SHELL`

- **Purpose**: Explicitly specify the current shell
- **Default**: Auto-detected
- **Example**:
  ```bash
  VP_SHELL=bash vp env print
  ```

### `VP_BYPASS`

- **Purpose**: Bypass Vite+ shim and use system tool directly
- **Values**: `PATH`-style list of directories to bypass
- **Default**: None
- **Example**:
  ```bash
  VP_BYPASS=/usr/local/bin node -v
  ```

### Internal (do not set manually)

These variables are set automatically by Vite+ during runtime and should not be configured manually:

- `VP_TOOL_RECURSION` — Recursion guard for `vp env exec` (prevents infinite shim loops)
- `VP_ACTIVE_NODE` — Records the active Node.js version (set by shim dispatch)
- `VP_RESOLVE_SOURCE` — Records how the Node.js version was resolved (set by shim dispatch)
- `VP_SHIM_TOOL` — Indicates which tool is being shimmed (set by shell wrapper scripts)
- `VP_SHIM_WRAPPER` — Windows shim wrapper flag (set by Windows shim wrappers)
- `VP_CLI_BIN` — Path to the `vp` binary (passed to JS scripts for CLI commands)
- `VP_GLOBAL_VERSION` — Global CLI version (passed from Rust binary to JS for `--version` display)

## TLS/CA Configuration

### `SSL_CERT_FILE`

- **Purpose**: Path to PEM bundle of extra CA certificates
- **Default**: System trust store
- **Example**:
  ```bash
  export SSL_CERT_FILE=/path/to/custom-ca.pem
  ```

### `NODE_EXTRA_CA_CERTS`

- **Purpose**: Path to PEM bundle of extra CA certificates (Node.js convention)
- **Default**: System trust store
- **Example**:
  ```bash
  export NODE_EXTRA_CA_CERTS=/path/to/custom-ca.pem
  ```

### `VP_INSECURE_TLS`

- **Purpose**: Disable HTTPS certificate verification
- **Values**: Any non-empty value (`1`, `true`, `yes`)
- **Default**: None (verification enabled)
- **Warning**: Diagnostic escape hatch only. Do not use in production. The effect is limited to the current process/command only; remove the variable immediately after troubleshooting.
- **Example**:

  ```bash
  VP_INSECURE_TLS=1 vp env install 22
  ```

  ```cmd
  :: CMD
  set VP_INSECURE_TLS=1 && vp env install 22
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

### `VITE_UPDATE_TASK_TYPES`

- **Purpose**: Filter for update task types
- **Default**: None
- **Example**:
  ```bash
  VITE_UPDATE_TASK_TYPES=dependencies vp update
  ```

### `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR`

- **Purpose**: Override directory for global CLI JS scripts
- **Default**: Auto-detected
- **Example**:
  ```bash
  VITE_GLOBAL_CLI_JS_SCRIPTS_DIR=/path/to/scripts vp help
  ```

## Testing/Development

### `VP_TRAMPOLINE_PATH`

- **Purpose**: Override trampoline binary path for tests
- **Default**: Auto-detected from `current_exe()`
- **Example**:
  ```bash
  VP_TRAMPOLINE_PATH=/path/to/trampoline vp setup
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
- **Effect**: Used to resolve `~/.vite-plus` default path

## Precedence

1. CLI flags (highest priority)
2. Environment variables
3. Default values (lowest priority)

For example, `VP_VERSION=1.0.0 vp-setup.exe --version 2.0.0` will install version 2.0.0.

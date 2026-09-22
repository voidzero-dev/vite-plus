<script setup lang="ts">
import { getScrollOffset } from 'vitepress';
import { nextTick, onMounted, onUnmounted } from 'vue';

function openTarget() {
  let target: HTMLElement | null;
  try {
    target = document.getElementById(decodeURIComponent(location.hash.slice(1)));
  } catch {
    return;
  }
  if (!target?.closest('details')) return;

  for (let details = target.closest('details'); details; details = details.parentElement?.closest('details') ?? null) {
    details.open = true;
  }

  // VitePress cannot measure a heading inside closed details. Correct the scroll after revealing it.
  requestAnimationFrame(() => {
    if (!target.isConnected) return;
    const top = window.scrollY + target.getBoundingClientRect().top - getScrollOffset()
      + Number.parseInt(window.getComputedStyle(target).paddingTop, 10);
    window.scrollTo(0, top);
  });
}

function onAnchorClick(event: MouseEvent) {
  if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
  const link = event.target instanceof Element ? event.target.closest('a') : null;
  // Clicking the current hash again does not emit hashchange.
  if (link?.href === location.href) openTarget();
}

onMounted(async () => {
  window.addEventListener('hashchange', openTarget);
  document.addEventListener('click', onAnchorClick);
  await nextTick();
  openTarget();
});

onUnmounted(() => {
  window.removeEventListener('hashchange', openTarget);
  document.removeEventListener('click', onAnchorClick);
});
</script>

# Global CLI

The global CLI is a standalone `vp` binary for machine-level runtime and package management. It includes a Vite+ toolchain, does not require Node.js to be installed first, and can be used without adding `vite-plus` to a project.

Choose the global CLI when you want one command available across projects for any combination of:

- managing Node.js and package-manager versions
- selecting and downloading package managers
- installing dependencies and running package binaries
- running `package.json` scripts and cached workspace tasks
- using the Vite+ frontend toolchain without pinning it in every project

Installing the global CLI does not require you to adopt the project-local package. You can use it only for runtime management, package management, and the task runner if that is all you need.

## Install

::: code-group

```bash [macOS / Linux]
curl -fsSL https://vite.plus | bash
```

```powershell [Windows]
irm https://vite.plus/ps1 | iex
```

:::

On Windows, you can instead download and run [`vp-setup.exe`](https://setup.viteplus.dev).

After installation, open a new shell and run:

```bash
vp help
```

When you enable environment management during installation, Vite+ records managed mode for Node.js and the npm, pnpm, Yarn, and Bun shims. Run `vp env off` to prefer system tools, or scope the change with `vp env off node` or `vp env off pm`.

::: details Installer Environment Variables & Options

The Vite+ installers (`vp-setup.exe`, `install.ps1`, and `install.sh`) and the installed `vp` CLI read the environment variables below.

### Installation Variables

These variables control the installer scripts and the standalone Windows installer (`vp-setup.exe`).

#### `VP_VERSION`

- **Purpose**: Version to install
- **Default**: `latest`
- **CLI equivalent**: `--version`
- **Note**: Vite+ 0.2.x and earlier do not support the split directory layout. The installer always puts these releases in the monolithic root (`VP_HOME` or `~/.vite-plus`). This rule also applies to a fresh machine. The installer checks the downloaded binary and prints a notice.
- **Example**:

  ```bash
  # Unix
  curl -fsSL https://vite.plus | VP_VERSION=1.2.3 bash
  ```

  ```powershell
  # PowerShell
  $env:VP_VERSION = "1.2.3"; irm https://vite.plus/ps1 | iex
  ```

#### `VP_HOME`

- **Purpose**: Optional pin for the single-root layout. Set it to an absolute path. Vite+ then puts bin, data, cache, config, and state under that directory. The installed CLI reads the same variable. See [Environment](/guide/env).
- **Default**: unset. Vite+ reuses an existing install in `~/.vite-plus` on Unix or `%USERPROFILE%\.vite-plus` on Windows. The directory must contain a `current` link. Otherwise, a fresh install uses the split platform layout. On Unix, it uses `~/.local/share/vite-plus` and its Vite+-owned `bin` subdirectory. On Windows, it uses `%LOCALAPPDATA%\vite-plus\data` and `%LOCALAPPDATA%\vite-plus\bin`.
- **Example**:

  ```bash
  # Unix
  curl -fsSL https://vite.plus | VP_HOME=/opt/vite-plus bash
  ```

  ```powershell
  # PowerShell
  $env:VP_HOME = "D:\vite-plus"; irm https://vite.plus/ps1 | iex
  ```

#### `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`

- **Purpose**: Internal absolute directory overrides for integrations that must pin a split install. Set all three variables together. The installer rejects an incomplete group. Vite+ ignores the group when `VP_HOME` is set or when it reuses an existing `~/.vite-plus` install.
- **Default**: unset (XDG / platform defaults)
- **Persistence**: The generated environment file does not export these variables. An integration that uses them must provide the complete group to each Vite+ process.
- **Example**:

  ```bash
  export VP_DATA_DIR=$HOME/vite-plus-data
  export VP_BIN_DIR=$VP_DATA_DIR/bin
  export VP_CACHE_DIR=$HOME/.cache/vite-plus
  curl -fsSL https://vite.plus | bash
  ```

#### `NPM_CONFIG_REGISTRY`

- **Purpose**: Custom npm registry URL
- **Default**: `https://registry.npmjs.org`
- **CLI equivalent**: `--registry`
- **Example**:
  ```bash
  curl -fsSL https://vite.plus | NPM_CONFIG_REGISTRY=https://registry.npmmirror.com bash
  ```

#### `VP_NODE_MANAGER`

- **Purpose**: Control Node.js version manager setup during installation.
- **Values**: `yes` or `no`
- **Default**: Auto-detected
- **CLI equivalent**: `--no-node-manager` (inverted)
- **Example**:
  ```bash
  # Skip Node.js manager setup in CI
  curl -fsSL https://vite.plus | VP_NODE_MANAGER=no bash
  ```

#### `VP_PM_MANAGER`

- **Purpose**: Set the management preference for all four package-manager families: npm, pnpm, Yarn, and Bun.
- **Values**: `yes` uses Vite+ management; `no` prefers system tools, with managed tools as a fallback when a system tool is unavailable.
- **Default**: Unset. The installer's combined Node.js and package-manager choice remains the default. With the script installers, setting only `VP_NODE_MANAGER` preserves existing package-manager preferences.

#### `VP_NPM_MANAGER` / `VP_PNPM_MANAGER` / `VP_YARN_MANAGER` / `VP_BUN_MANAGER`

- **Purpose**: Set the management preference for an individual package-manager family. Each variable overrides `VP_PM_MANAGER` for that family.
- **Values**: `yes` or `no`, with the same meaning as `VP_PM_MANAGER`.
- **Default**: Unset (use `VP_PM_MANAGER`, then the combined installer choice, or preserve the existing preference).
- **Example**:

  ```bash
  # Keep system Node.js and package managers, but let Vite+ manage pnpm.
  curl -fsSL https://vite.plus | VP_NODE_MANAGER=no VP_PM_MANAGER=no VP_PNPM_MANAGER=yes bash
  ```

These management variables are installation choices, saved in Vite+'s config. The interactive prompt still controls both Node.js and package managers; explicit package-manager variables override that combined choice. The standalone `vp-setup` installer uses its existing combined option as the default for both variables, in interactive and silent installations alike. In-place upgrades preserve the saved choices. Unrecognized values are ignored. They select management behavior, not package-manager versions, and do not prevent the installer from creating shims. Older releases installed through the legacy installer retain their original behavior.

#### `VP_PR_VERSION`

- **Purpose**: Install a preview build from a pull request or commit SHA
- **Values**: PR number or commit SHA
- **Default**: None
- **Details**: [Global `vp` Preview](/guide/upgrade#global-vp-preview)

#### Development variables

Use `VP_LOCAL_TGZ` and `VP_LOCAL_BINARY` when you develop Vite+ itself. `VP_LOCAL_TGZ` specifies a local `vite-plus.tgz` file. `VP_LOCAL_BINARY` specifies a local `vp` binary. The installers use these files for the local build. They use `VP_DUMP_DIRS=1` to get the layout mode and all five `EnvConfig` category roots from the selected binary. They do not resolve the directory variables. The installers set `VP_INSTALL_STOP`; do not set it manually.

### Runtime Variables

These variables configure the installed Vite+ CLI. `VP_HOME` (above) also applies at runtime.

#### `VP_NODE_DIST_MIRROR`

- **Purpose**: Node.js distribution mirror URL
- **Default**: `https://nodejs.org/dist`
- **Details**: [Custom Node.js Mirror](/guide/env#custom-node-js-mirror)

#### `VP_NODE_VERSION`

- **Purpose**: Override Node.js version
- **Default**: None (auto-detected)
- **Example**:
  ```bash
  # Run a command with a specific Node.js version
  VP_NODE_VERSION=22 vp env exec node -v
  ```

#### `VP_PACKAGE_MANAGER`

- **Purpose**: Override the selected package manager and version
- **Default**: None (resolved from the project or global default)
- **Format**: `npm|pnpm|yarn|bun@<version>`
- **Example**:
  ```bash
  VP_PACKAGE_MANAGER=pnpm@10.18.0 vp install
  ```

#### `VP_NODE_SKIP_SIGNATURE_VERIFY`

- **Purpose**: Skip PGP signature verification of Node.js downloads
- **Values**: Any non-empty value
- **Default**: None (verification enabled)
- **Details**: [Node.js Signature Verification](/guide/env#node-js-signature-verification)

#### `VP_DOWNLOAD_TIMEOUT`

- **Purpose**: Per-request timeout, in seconds, for large downloads such as Node.js runtimes and package-manager tarballs
- **Values**: Positive integer, at most `86400` (24 hours); invalid values are ignored with a warning
- **Default**: `600` (10 minutes)
- **Example**:
  ```bash
  # Allow up to 30 minutes per download on a slow connection
  VP_DOWNLOAD_TIMEOUT=1800 vp env install 22
  ```

#### `VP_SHELL`

- **Purpose**: Specify the current shell
- **Default**: Auto-detected
- **Example**:
  ```bash
  VP_SHELL=bash vp env print
  ```

#### `VP_BYPASS`

- **Purpose**: Bypass the Vite+ shim and use the system tool
- **Values**: `PATH`-style list of directories to bypass
- **Default**: None
- **Example**:
  ```bash
  VP_BYPASS=/usr/local/bin node -v
  ```

#### Internal variables

Vite+ sets additional `VP_*` variables during shim dispatch and shell integration (recursion guards, active-version records, wrapper flags); do not set them manually.

### TLS/CA Configuration

#### `SSL_CERT_FILE` / `NODE_EXTRA_CA_CERTS`

- **Purpose**: Path to PEM bundle of extra CA certificates (`NODE_EXTRA_CA_CERTS` is the Node.js convention)
- **Default**: System trust store
- **Example**:
  ```bash
  export SSL_CERT_FILE=/path/to/custom-ca.pem
  ```

#### `VP_INSECURE_TLS`

- **Purpose**: Disable HTTPS certificate verification
- **Values**: Any non-empty value (`1`, `true`, `yes`)
- **Default**: None (verification enabled)
- **Warning**: Diagnostic escape hatch only; do not use in production
- **Example**:
  ```bash
  VP_INSECURE_TLS=1 vp env install 22
  ```

### Logging and Debugging

#### `VP_LOG`

- **Purpose**: Log filter string for `tracing_subscriber`
- **Installer behavior**: When `CI=true`, `install.sh` hides shell file errors. Set `VP_LOG=trace` to show these errors.
- **Default**: None
- **Example**:
  ```bash
  VP_LOG=debug vp dev
  VP_LOG=vt=trace vp build
  ```

#### `VP_DEBUG_SHIM`

- **Purpose**: Enable debug output for shim dispatch
- **Values**: Any non-empty value
- **Default**: None
- **Example**:
  ```bash
  VP_DEBUG_SHIM=1 node -v
  ```

### Standard Environment Variables

Vite+ also respects these standard environment variables:

#### Nushell and XDG directories

If you customize `XDG_DATA_HOME` or `XDG_CONFIG_HOME`, set them **before starting Nushell**, through your terminal application, operating system, or parent shell. This is a [Nushell startup requirement](https://www.nushell.sh/book/configuration.html#changing-default-directories); setting them only in `config.nu` or `env.nu` does not configure the running session's startup directories.

Assignments in those files still affect child processes. The Vite+ installer starts a child Nushell to locate its vendor autoload directory, so it can write `vite-plus.nu` to a directory that normal new sessions do not read. Installation can succeed while `vp` remains unavailable in those sessions.

If this happens, open your Nushell configuration with `config nu` and add a `source` line pointing to the installed Vite+ `env.nu` file. For a default fresh macOS or Linux installation without a custom `XDG_CONFIG_HOME`, use:

```nu
source ~/.config/vite-plus/env.nu
```

For a custom `XDG_CONFIG_HOME`, use the absolute path to `<XDG_CONFIG_HOME>/vite-plus/env.nu` as resolved during installation. For an installation under `VP_HOME` or an existing `~/.vite-plus` installation, use `<VP_HOME>/env.nu` or `~/.vite-plus/env.nu` instead. Replace placeholders with actual paths and quote paths containing spaces. Open a new Nushell session and run `vp help` to verify the configuration.

#### `CI`

- **Purpose**: Indicates running in CI environment
- **Effect**: Enables silent mode (`--yes`) for installers

#### `NO_COLOR`

- **Purpose**: Disable colored output
- **Effect**: Disables ANSI color codes

#### `HOME` / `USERPROFILE`

- **Purpose**: User home directory
- **Effect**: Base for the existing-install probe (`~/.vite-plus`) and for split platform defaults

### Precedence

1. CLI flags (highest priority)
2. Environment variables
3. Default values (lowest priority)

For example, `VP_VERSION=1.0.0 vp-setup.exe --version 2.0.0` installs version 2.0.0.

:::

### Homebrew

For a Homebrew installation, the first `vp` command sets up your shell, shims, and environment-management preferences. It reuses Homebrew's binary and bundled JavaScript. Setup stores its completion state in your user directories and does not need write access to the Homebrew prefix.

Later commands reuse that setup while the installed binary remains unchanged. Generated shims follow Homebrew's public `vp` entrypoint when Homebrew replaces a version. Setup preserves your saved management preferences during this replacement.

To prefer your existing Node.js and package managers during the first run, use:

```bash
VP_NODE_MANAGER=no VP_PM_MANAGER=no vp help
```

After setup, use `vp env off` to change this preference. Commands that need missing runtimes or project dependencies can still download them.

Use Homebrew to [upgrade](/guide/upgrade#homebrew) or [remove](/guide/implode#homebrew) its package.

`vp env doctor` identifies the Homebrew installation and its binary path. It checks the `vp` command and the user shim directory on `PATH` separately. If only the shim directory is missing, follow its shell setup instructions to enable the shims.

## Use It Without a Local Package

The global installation is enough for runtime, package-manager, and task-runner workflows:

```bash
vp env pin lts       # Pin and install Node.js for this project
vp install           # Use the package manager declared by the project
vp run build         # Run a package.json script or configured task
vp dlx create-vite   # Download and run a package binary
```

You do not need a local `vite-plus` dependency to run existing `package.json` scripts. Add the [project-local CLI](/guide/local-cli) when you want the frontend toolchain version recorded in the project's manifest and lockfile.

## Use Both CLIs Together

The global CLI and the project-local `vite-plus` package work together. You keep using the same `vp` command, while each project can choose its own toolchain version.

For development commands such as `vp dev`, `vp build`, `vp test`, and `vp run`, the global CLI delegates to the project's installed version when available:

| Current project                     | Toolchain used by `vp`            |
| ----------------------------------- | --------------------------------- |
| Has `vite-plus` installed locally   | The project's installed toolchain |
| Does not have `vite-plus` installed | The globally installed toolchain  |

In a monorepo, the local installation can be shared at the workspace root. You do not need to install `vite-plus` separately in every package.

For example, if a project has Vite+ version A installed and your global installation is version B, `vp build` uses version A's toolchain. Upgrading the global installation does not change that project's installed toolchain.

Package-manager commands such as `vp install` and `vp add` use the global CLI. Commands for managing your environment or global installation, such as `vp env`, `vp upgrade`, and `vp implode`, also stay with the global CLI regardless of the project's version.

To see which toolchain is selected for your current project, run `vp toolchain`. Use `vp toolchain --global` to inspect the global installation.

## Next Steps

- [Environment](/guide/env) covers Node.js and package-manager selection, pinning, shims, and managed installations.
- [Package Management](/guide/install) covers pnpm, npm, Yarn, and Bun workflows.
- [Run](/guide/run) covers package scripts and cached workspace tasks.
- [Upgrading Vite+](/guide/upgrade) explains global CLI upgrades. See [Update Vite+](/guide/upgrade-project) for project-local upgrades.
- [Removing Vite+](/guide/implode) removes the global binary and its managed data.

::: details Platform support

Prebuilt binaries are distributed for:

- Linux x64 and arm64 with glibc
- Windows x64 and arm64
- macOS x64 and arm64
- Linux x64 and arm64 with musl

If a prebuilt binary is not available for your platform, installation fails with an error. On Alpine Linux, install `libstdc++` before using the managed [unofficial Node.js builds](https://unofficial-builds.nodejs.org/):

```sh
apk add libstdc++
```

:::

# Homebrew tap

The official tap supports macOS and glibc Linux on ARM64 and x64. Homebrew manages the native `vp` executable; `vp` manages its JavaScript dependencies, downloaded Node.js versions, package managers, and settings in your user directories.
Stable installation downloads the executable and its dependency lockfile from GitHub Releases. This step needs no existing Node.js or package manager installation, or npm registry access.

## Install

Install the tap:

```bash
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew install voidzero-dev/vite-plus/vp
vp
```

The explicit repository URL is required because the formula is part of the Vite+ repository.
If you already use Vite+, follow the [migration steps](#switch-an-existing-installation).

## First use

On first use, `vp` installs its matching CLI dependencies and configures your shell and tool-management preferences. This step needs access to Node.js downloads and your npm registry.
Setup uses the release's pinned Node.js and pnpm versions, even if you choose to prefer system tools. A compatible [`VP_SETUP_NODE_VERSION`](/guide/global-cli#vp-setup-node-version) can override the installation runtime without changing Node.js selection for your projects.
`vp` stores dependencies in `<DATA>/cli-packages/<version>/<platform>`.
`<DATA>` comes from the [Vite+ directory configuration](/guide/global-cli#installation-variables).

Open a new terminal after setup so the shims are available on `PATH`. Later invocations reuse the completed setup.
Use [`vp env on` and `vp env off`](/guide/env) to change which tools `vp` manages.

## Custom npm registry

First-run setup reads `~/.npmrc`, including npm-compatible authentication:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${NPM_TOKEN}
```

Export `NPM_TOKEN` before running `vp`. The native pnpm downloader and pnpm dependency installation both use this configuration.
You can select another file with `NPM_CONFIG_USERCONFIG` or override the registry with `NPM_CONFIG_REGISTRY`.
Relative user-config paths resolve from your current directory. No Homebrew-specific npm variables are required.

Authentication failures stop setup. Fix the configuration and run `vp` again; setup preserves your management choices.

## Recover from failed setup

Run `vp env doctor` to inspect the executable, dependency directory, and shims. Its CLI dependency check reads local files and does not install or repair dependencies. Other checks can fetch Node.js version metadata.

Fix the reported configuration, access, or disk-space problem, then run `vp --help` to retry setup.
For a lockfile error, the [installer recovery option](/guide/global-cli#vp-setup-skip-install-lockfile) also works with Homebrew:

```bash
VP_SETUP_SKIP_INSTALL_LOCKFILE=1 vp --help
```

This resolves dependencies without the published lockfile. A later invocation without the override restores the normal lockfile checks and can reinstall those dependencies.
If pnpm fails, the reported log contains its exit code and recognized error codes, without raw package-manager output.
Configured tool shims can run cached Node.js, package managers, and global packages independently of CLI dependency repair.

## Switch an existing installation

Keep your existing directory settings. Migration preserves your preferences, runtimes, and global packages, and updates their shims to Homebrew's public `vp` command.
Do not run `vp implode`: it removes that user data. Open a new terminal after switching.

### Switch from the script installer

Run the `brew tap` and `brew install` commands above. Then invoke Homebrew's executable directly, so an existing script-install shim cannot take precedence:

```bash
"$(brew --prefix)/bin/vp" env setup --refresh
```

### Switch from Homebrew Core

The `vite-plus` formula provides the same commands and must be removed before installing the tap:

```bash
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew uninstall vite-plus
brew install voidzero-dev/vite-plus/vp
"$(brew --prefix)/bin/vp" env setup --refresh
```

## Upgrade or remove

```bash
brew upgrade voidzero-dev/vite-plus/vp
```

Run `vp` after upgrading to prepare its matching dependencies. Setup preserves your preferences.
`vp upgrade` identifies the official tap and directs you to `brew upgrade voidzero-dev/vite-plus/vp`.

For complete removal, remove your user data before uninstalling the executable:

```bash
vp implode
brew uninstall voidzero-dev/vite-plus/vp
```

`brew uninstall` alone keeps user data. On a shared machine, each operating-system account can remove its own data with `vp implode` before the executable is removed.

## Test a preview build

Set `HOMEBREW_VP_PR_VERSION` to a PR number or full commit SHA. The PR must have a published `preview-build` that includes Homebrew tap support. Use a full SHA to keep installation and verification on the same commit:

```bash
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
HOMEBREW_VP_PR_VERSION="<full-commit-sha>" brew install voidzero-dev/vite-plus/vp
HOMEBREW_VP_PR_VERSION="<full-commit-sha>" brew test voidzero-dev/vite-plus/vp
```

Use `brew reinstall` instead of `brew install` to replace an installed tap build. If the `vite-plus` formula is installed, run `brew uninstall vite-plus` first; this keeps your user data.
`brew test` uses temporary user data and does not change your shell configuration. Run `vp` afterward to set up the preview for your account. If switching from the script installer, [refresh its shims](#switch-from-the-script-installer) to use the Homebrew executable.

Homebrew downloads only the native preview package from the registry bridge and verifies its checksum. It resolves PR numbers to the latest published commit, which can differ from the current PR head.
First launch installs the matching dependencies from the bridge. An explicit `NPM_CONFIG_REGISTRY` must point to a registry that also serves those preview packages.

The selector is specific to this tap. It does not change other Homebrew formulae, and no npm client is needed during `brew install`.
To test changes to the formula itself, first check out the PR in the tap repository. Set `HOMEBREW_NO_AUTO_UPDATE=1` for these commands to keep that checkout.

To return to a stable release, clear the overrides, restore the tap's `main` branch if you changed its checkout, and update it before reinstalling:

```bash
unset HOMEBREW_VP_PR_VERSION HOMEBREW_NO_AUTO_UPDATE
git -C "$(brew --repository voidzero-dev/vite-plus)" switch main
brew update
brew reinstall voidzero-dev/vite-plus/vp
```

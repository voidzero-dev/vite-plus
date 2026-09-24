# Homebrew tap

::: warning Availability
Stable installation stays disabled until the first release with per-user Homebrew setup is published. Preview builds can be tested with the selector below.
Use the [script installer](/guide/global-cli) in the meantime.
:::

The official tap installs the native `vp` CLI from GitHub Releases on macOS and glibc Linux, for ARM64 and x64.
Homebrew needs no Node.js, pnpm, or npm registry access to install it.

When the tap becomes available, run:

```bash
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew install voidzero-dev/vite-plus/vp
vp
```

The explicit repository URL is required. The formula is part of the Vite+ repository.

On first use, `vp` downloads Node.js LTS, pinned pnpm, and matching CLI dependencies.
It stores dependencies in `<DATA>/cli-packages/<version>/<platform>` and sets up your preferences and shims.
`<DATA>` comes from the [Vite+ directory configuration](/guide/global-cli#installation-variables).
The native executable stays under Homebrew's control. Later invocations reuse the completed setup.
`vp env doctor` shows both locations.

## Custom npm registry

First-run setup reads `~/.npmrc`, including npm-compatible authentication:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${NPM_TOKEN}
```

Export `NPM_TOKEN` before running `vp`. Both npm, which downloads pnpm, and pnpm, which installs dependencies, use this configuration.
You can select another file with `NPM_CONFIG_USERCONFIG` or override the registry with `NPM_CONFIG_REGISTRY`.
Relative user-config paths resolve from your current directory. No Homebrew-specific npm variables are required.

Authentication failures stop setup. Fix the configuration and run `vp` again; setup preserves your management choices.
Failure logs record the exit code without registry output, which can contain credentials.

## Switch from the script installer

Install the tap with the commands above. Then invoke Homebrew's executable directly, so an existing script-install shim cannot take precedence:

```bash
"$(brew --prefix)/bin/vp" env setup --refresh
hash -r
```

Use the same directory overrides as your existing installation.
Setup preserves your preferences, runtimes, and global packages, and updates their shims to Homebrew's public `vp` command.
Do not run `vp implode` to migrate: it removes that user data.

## Upgrade or remove

```bash
brew upgrade voidzero-dev/vite-plus/vp
```

The first invocation of the new version installs its matching dependencies and preserves your preferences.
`vp upgrade` identifies the official tap and directs you to `brew upgrade voidzero-dev/vite-plus/vp`.

For complete removal, remove your user data before uninstalling the executable:

```bash
vp implode
brew uninstall voidzero-dev/vite-plus/vp
```

`brew uninstall` alone keeps user data. Each user can remove their own data with `vp implode` before the shared executable is removed.

## Test a preview build

Set `HOMEBREW_VP_PR_VERSION` to a PR number or full commit SHA. The PR must have a published `preview-build` that includes Homebrew tap support.

```bash
HOMEBREW_VP_PR_VERSION="<pr-or-sha>" brew install voidzero-dev/vite-plus/vp
HOMEBREW_VP_PR_VERSION="<pr-or-sha>" brew test voidzero-dev/vite-plus/vp
```

Use `brew reinstall` instead of `brew install` to replace an installed build. Use a full SHA to keep installation and verification on the same commit.
`brew test` uses temporary user data and does not change your shell configuration.

Homebrew downloads only the native preview package from the registry bridge and verifies its checksum. It resolves PR numbers to the latest published commit, which can differ from the current PR head.
First launch installs the matching dependencies from the bridge. An explicit `NPM_CONFIG_REGISTRY` must point to a registry that also serves those preview packages.

The selector is specific to this tap. It does not change other Homebrew formulae, and no npm client is needed during `brew install`.
To test changes to the formula itself, first check out the PR in the tap repository. Set `HOMEBREW_NO_AUTO_UPDATE=1` for these commands to keep that checkout.
To return to a stable release once available, run `brew reinstall voidzero-dev/vite-plus/vp` without the selector.

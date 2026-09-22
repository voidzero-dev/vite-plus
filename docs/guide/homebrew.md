# Homebrew tap

::: warning Availability
The official tap is not available yet. Its formula stays disabled until the first release with per-user Homebrew setup is published.
Use the [script installer](/guide/global-cli) or Homebrew core in the meantime.
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

## Switch an existing installation

For a Homebrew core installation, remove the core formula first:

```bash
brew uninstall vite-plus
```

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
`vp upgrade` detects the owning formula and shows the appropriate Homebrew command.

For complete removal, remove your user data before uninstalling the executable:

```bash
vp implode
brew uninstall voidzero-dev/vite-plus/vp
```

`brew uninstall` alone keeps user data. Each user can remove their own data with `vp implode` before the shared executable is removed.

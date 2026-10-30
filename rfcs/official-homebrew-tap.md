# RFC: Official Homebrew tap

Status: Proposed, with implementation in this PR. Stable installation stays
disabled until maintainers publish a compatible release and merge its formula update.

## Motivation

Issue [#1171](https://github.com/voidzero-dev/vite-plus/issues/1171) requested
Homebrew distribution for atomic Linux desktops and enterprise management.
The [core formula](https://github.com/Homebrew/homebrew-core/blob/main/Formula/v/vite-plus.rb)
requires Rust and JavaScript builds when a bottle is unavailable. Issue
[#2719](https://github.com/voidzero-dev/vite-plus/issues/2719) also exposed a
mismatch between Homebrew installation and first-run setup, addressed by
[#2729](https://github.com/voidzero-dev/vite-plus/pull/2729).

The official tap installs the native CLI from GitHub Releases and reuses the
script installer's dependency setup with the user's npm configuration.

## Proposal

Keep [`HomebrewFormula/vp.rb`](../HomebrewFormula/vp.rb) in this repository.
Once the tap is available, install it with:

```sh
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew install voidzero-dev/vite-plus/vp
vp
```

The repository URL is required because its name has no `homebrew-` prefix.
The qualified formula name distinguishes the tap from Core's `vite-plus` formula
and `vp` alias.

| Manager  | Files                                                                                                     |
| -------- | --------------------------------------------------------------------------------------------------------- |
| Homebrew | Native `vp` executable, companion lockfile, and public command links.                                     |
| `vp`     | CLI dependencies, downloaded Node.js and package managers, settings, and shims in the user's directories. |

Here, the user is the operating-system account running `vp`.
Node.js and package-manager commands can use managed or system-first mode.
These choices do not disable the runtime and pinned pnpm used to install CLI
dependencies. Existing package formats and Homebrew Core's build recipe stay unchanged.

The scope is stable releases on macOS and glibc Linux, for ARM64 and x64.
Installation needs GitHub access; dependency setup also needs the configured npm
registry and runtime download sources. Preview builds use the registry bridge.

## Installation flow

The examples show installation of version `A`, then an upgrade to `B`.
`<BREW>` is Homebrew's prefix. `VpDirs` resolves `<DATA>`, `<BIN>`, `<CONFIG>`,
and `<STATE>` from the [directory configuration](directory-layout.md).
The trees omit unrelated files; receipt names are illustrative.

### 1. Register the tap

`brew tap` clones the repository and makes its formula available.

```text
<Homebrew tap checkout>/
└── HomebrewFormula/vp.rb
```

### 2. Install the native CLI

`brew install` checks the release archive's SHA-256 checksum, installs the binary
and lockfile in the Cellar, and links `vp`, `vpr`, and `vpx`. It does not run
Node.js, npm, pnpm, or user setup.

```text
<BREW>/
├── Cellar/vp/A/
│   ├── bin/
│   │   ├── vp -> ../libexec/vp
│   │   ├── vpr -> vp
│   │   └── vpx -> vp
│   ├── libexec/
│   │   ├── vp
│   │   └── vp-install-lock.yaml
│   └── INSTALL_RECEIPT.json
└── bin/
    ├── vp  -> ../Cellar/vp/A/bin/vp
    ├── vpr -> ../Cellar/vp/A/bin/vpr
    └── vpx -> ../Cellar/vp/A/bin/vpx
```

### 3. Prepare dependencies on first use

Running `vp` prepares Node.js and pinned pnpm, copies
`vp-install-lock.yaml`, and installs `vite-plus@A` with `--frozen-lockfile`.
By default, it uses the Node.js version embedded in the CLI release from
`.node-version`; a compatible `VP_SETUP_NODE_VERSION` can override it.
This installation runtime does not change Node.js selection for projects or tool shims.
The [shared installer](../crates/vp_setup/src/install.rs) saves dependencies by
CLI version and platform. Setup saves management choices before downloads and
serializes concurrent attempts, so a failed attempt can retry without asking again.

`vp env doctor`, `vp implode`, and `vp upgrade` bypass dependency setup and
remain available after installation failures. Doctor checks dependency completeness
without downloading anything. Configured tool shims can run independently of the
CLI package, so an interrupted CLI installation does not block cached tools.
CLI invocations, including `vpr`, `vpx`, `vp --help`, and `vp --version`, still run
setup when needed. Explicit installer requests also run setup.

```text
<CONFIG>/config.json                 # saved before downloads

<DATA>/
├── js_runtime/node/<node-version>/
│   ├── bin/node
│   └── lib/node_modules/npm/        # included in the Node.js distribution
├── package_manager/pnpm/<pnpm-version>/pnpm/
└── cli-packages/A/<platform>/
    ├── bin/vp-install-lock.yaml      # copy of the published lockfile
    ├── package.json
    ├── pnpm-lock.yaml
    └── node_modules/
        ├── .vp-install-complete     # shared receipt for the installed lockfile
        ├── vite-plus/
        └── ...                     # production dependencies
```

### 4. Complete user setup

After installation succeeds, setup writes shell environment files, shims, and
a setup receipt. Shims point through Homebrew's stable public command. This
example uses managed Node.js and system-first npm:

```text
<CONFIG>/
├── config.json
└── env                             # Bash/Zsh; other shell files omitted

<BIN>/
├── vp   -> <BREW>/bin/vp
├── node -> <BREW>/bin/vp
└── ...                             # vpr, vpx, and other managed shims

<DATA>/fallback-bin/
├── npm  -> <BREW>/bin/vp
├── npx  -> <BREW>/bin/vp
└── ...                             # other system-first shims

<STATE>/self-setup/
└── <receipt-for-A>.json
```

`<BIN>` comes first on `PATH`; `<DATA>/fallback-bin` comes last so system tools
take precedence over fallback shims. The executable stays in the Cellar.
The [JavaScript resolver](../crates/vp_global_cli/src/js_executor.rs) normally
prefers project-local `vite-plus` and uses the matching `cli-packages` directory
as its global fallback.

Later invocations reuse setup while its completion checks remain valid. The
checks compare the shared install receipt with the companion and working
lockfiles, and check the CLI package version and entrypoint. Missing or
inconsistent state triggers repair when the CLI runs. Setup writes only to user directories.

### 5. Upgrade through Homebrew

`brew upgrade` installs version `B` and replaces its public links. User shims
continue to point through those links, even after Homebrew removes version `A`.

```text
<BREW>/Cellar/vp/B/
├── bin/                            # public vp, vpr, vpx links
├── libexec/                        # new vp and vp-install-lock.yaml
└── INSTALL_RECEIPT.json

<BREW>/bin/
├── vp  -> ../Cellar/vp/B/bin/vp
├── vpr -> ../Cellar/vp/B/bin/vpr
└── vpx -> ../Cellar/vp/B/bin/vpx
```

Running `vp` after the upgrade installs `B`'s dependencies and records completion.
The new binary supplies its release's default Node.js pin; the formula needs no
separate pin. Setup preserves preferences and reuses cached bootstrap tools that
match the selected versions. Recovery commands retain the setup bypass described above.

```text
<DATA>/cli-packages/B/<platform>/
├── bin/vp-install-lock.yaml
├── package.json
├── pnpm-lock.yaml
└── node_modules/
    ├── vite-plus/
    └── ...

<STATE>/self-setup/
└── <receipt-for-B>.json
```

The [self-setup code](../crates/vp_global_cli/src/self_setup.rs) keeps these
directories separate from the script installer's `current` link. Core continues
to use its bundled JavaScript; bare standalone binaries retain the managed
installation layout. Setup reuses the existing environment and shim code.

## Registry configuration

Dependency setup runs outside Homebrew's build sandbox. The native pnpm downloader
and pnpm read `~/.npmrc` or `NPM_CONFIG_USERCONFIG` with npm-compatible token and
basic authentication:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${NPM_TOKEN}
```

Export `NPM_TOKEN` for this example. `NPM_CONFIG_REGISTRY` overrides the registry;
relative user-config paths resolve before pnpm changes directories. The
[pnpm bootstrap](../crates/vp_pm_cli/src/package_manager/bootstrap.rs) uses the
shared downloader for metadata, integrity checks, caching, and locking.

Setup retains the shared installer's explicit lockfile bypass option. Authentication
failures stop setup without a public-registry fallback. Credentials stay out of
command arguments, logs, and receipts; no Homebrew-specific npm settings are needed.
Failure logs retain known error codes and fixed guidance for authentication,
lockfile, and disk-space errors, without saving raw package-manager output.

## Commands and migration

[Homebrew detection](../crates/vp_global_cli/src/homebrew.rs) reads the installation
receipt to distinguish the official tap, Core, and other taps. Notices name the
installed formula. For the official tap:

| Command                                    | Behavior                                                                                                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `vp upgrade`                               | Directs users to `brew upgrade voidzero-dev/vite-plus/vp`, including requests for explicit versions, force, or rollback. |
| `vp upgrade --check`                       | Directs users to `brew outdated voidzero-dev/vite-plus/vp`.                                                              |
| `vp env doctor`                            | Reports the Homebrew executable and checks CLI dependency completeness without downloading.                              |
| `vp implode`                               | Removes user-managed dependencies, runtimes, settings, and shims.                                                        |
| `brew uninstall voidzero-dev/vite-plus/vp` | Removes the Homebrew package and keeps user data.                                                                        |

Homebrew installations disable npm update checks for the CLI. Complete removal
requires `vp implode` before `brew uninstall`; migration must preserve user data.
The formula conflicts with Core's `vite-plus` because both provide the same commands.

Migration refreshes shims and preserves preferences, runtimes, and global packages.
The [Homebrew guide](../docs/guide/homebrew.md#switch-from-the-script-installer)
shows how to invoke Homebrew's binary when script-install shims take precedence.
Core's upgrade notice also explains how to switch to the tap.

## Repository and releases

The [release workflow](../.github/workflows/release.yml) advances the stable formula
after publication:

1. The [updater](../.github/scripts/update-homebrew-formula.mjs) checks the release
   tag for the tap formula, verifies all release archives, and waits for matching
   npm packages. It prepares one candidate with versioned URLs and checksums.
2. The [published-artifact checks](../.github/workflows/test-homebrew-published.yml)
   install that unchanged candidate on all four platforms. They check first use,
   then `fmt`, `lint`, and `build` without project-local dependencies.
3. After all four jobs pass, automation opens a PR with the same candidate artifact.
   It rejects the candidate if the formula or guide changed on `main` during testing.
   Maintainers review and merge the formula update to make it available.

Failures leave the previous formula available. The first compatible release removes
the disable gate through this process; older releases without the tap formula cannot
enable it. Enabled formula PRs repeat the published-artifact checks. Source-build
tests remain separate. Use a formula `revision` for recipe fixes and a product
release for CLI changes; formula-only merges do not trigger product releases.

`HOMEBREW_VP_PR_VERSION` selects a published preview by PR number or full commit
SHA. The formula downloads and verifies the native package from the registry bridge;
setup installs matching preview dependencies. See the
[preview instructions](../docs/guide/homebrew.md#test-a-preview-build).

Release maintainers own the formula through the existing repository review process.
Formula failures belong in this repository's issue tracker.

## Launch checks

The [installation tests](../.github/workflows/test-homebrew.yml) use isolated homes
and synthetic credentials. In addition to the published-artifact checks, cover:

- Installation without preinstalled Node.js or pnpm, or creation of Vite+ user data.
- Public and authenticated registries, token and basic authentication, config
  overrides, invalid credentials, and corrupt archives.
- Concurrent setup, receipt and lockfile repair, preserved preferences, and
  recovery commands after failed setup, without writes to the Cellar.
- Upgrade and old-keg removal with both shim directories, including `pn` and `pnx`.
- Independent account data, migration, `vp implode`, and Homebrew uninstall.

PR installation checks require the `test: install-e2e` label and cover macOS ARM64
and Linux ARM64/x64. Pushes to `main` do not run this workflow. Release verification
continues to cover all four supported platforms. PR tests receive no publishing credentials.

## Tradeoffs

Reusing the script installer avoids a separate dependency installation path, but
first use needs network access and stores dependencies separately for each OS account.
This change does not automatically prune old `cli-packages` versions.

Keeping the formula in this repository reuses its release and review process.
Homebrew supports the [`HomebrewFormula/` layout](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#creating-a-tap),
though `brew tap` also downloads unrelated project files.

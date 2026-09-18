# RFC: Vite+-managed Homebrew tap

Status: Draft for discussion. This RFC does not change installation behavior.

## Motivation

Issue [#1171](https://github.com/voidzero-dev/vite-plus/issues/1171) requested
Homebrew distribution for atomic Linux desktops and enterprise management.
The [core formula](https://github.com/Homebrew/homebrew-core/blob/main/Formula/v/vite-plus.rb)
requires Rust and JavaScript builds when a bottle is unavailable. Issue
[#2719](https://github.com/voidzero-dev/vite-plus/issues/2719) also exposed a
mismatch between Homebrew installation and first-run setup, addressed by
[#2729](https://github.com/voidzero-dev/vite-plus/pull/2729).

A project-maintained tap can reuse the npm packages used by
[`install.sh`](../packages/cli/install.sh), avoid compiling Vite+ from source,
and support enterprise npm registries.

## Proposal

Maintain `HomebrewFormula/vp.rb` in the existing `voidzero-dev/vite-plus`
repository. Once the tap is available, users install it with:

```sh
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew install voidzero-dev/vite-plus/vp
vp
```

The explicit repository URL is required because Homebrew otherwise looks for
`voidzero-dev/homebrew-vite-plus`. Use the qualified formula name to distinguish
it from core's `vite-plus` formula and `vp` alias.

Use npm as the primary distribution source. Download the native binary from
`@voidzero-dev/vite-plus-cli-<platform>` and install `vite-plus` at the same
version with its production dependencies. Keep the published package formats
unchanged.

Follow the script installer's sequence: download the binary, install JavaScript
dependencies, then configure the user environment. Homebrew owns the first two
steps and all package upgrades and removal. The first `vp` invocation runs the
existing per-user setup. Running `install.sh` unchanged would instead place
files in the user's managed installation.

The initial scope is stable releases on macOS and glibc Linux, for ARM64 and
x64. Launch each target only after its installation tests pass.

## Installation design

The formula uses a release-generated manifest, dependency lock, and package
checksums:

1. Download the native CLI, pinned pnpm, and locked production dependencies as
   [Homebrew resources](https://docs.brew.sh/Formula-Cookbook#language-specific-dependencies).
   Include the target's optional native packages.
2. Point a working copy of the lockfile at the verified local tarballs. Retain
   its package versions and integrity values.
3. Use a Homebrew Node.js build dependency to run the pinned pnpm with
   `--offline --frozen-lockfile --prod --ignore-scripts`.
4. Install the executable into `prefix/bin/vp` and the dependency tree into
   `prefix/node_modules`. Add `vpr` and `vpx` links to `vp`.

Pin pnpm to the version used by the existing
[dependency installer](../crates/vp_setup/src/install.rs). Copy dependency files
from an isolated pnpm store; installed links must stay within the package.
The CLI must work after the build directory and store are removed.

```text
<Cellar>/vp/<version>/
├── bin/                    # vp, vpr, vpx
├── node_modules/           # vite-plus and its production dependencies
└── INSTALL_RECEIPT.json     # Homebrew ownership record
```

The existing [external setup](../crates/vp_global_cli/src/self_setup.rs) and
[JavaScript resolver](../crates/vp_global_cli/src/js_executor.rs) support this
layout. First use creates each user's settings, shims, and setup receipt without
copying the package or writing into the Cellar. Upgrades must preserve those
settings, and shims must survive removal of the old package version.

Runtime Node.js selection remains unchanged: system-first mode can use Node.js
on `PATH`, while managed mode can download a separate runtime. The Node.js build
dependency does not override that preference.

## Custom registries and authentication

Choose the default registry in this order:

1. A nonempty `HOMEBREW_NPM_CONFIG_REGISTRY`.
2. `registry` in `~/.npmrc`.
3. `https://registry.npmjs.org`.

Honor scoped registry entries such as `@voidzero-dev:registry` ahead of the
default for matching packages. Read the user's `.npmrc`, not a project's file.
These settings belong to the proposed tap; they are not built-in Homebrew
registry settings.

Homebrew's [launcher](https://github.com/Homebrew/brew/blob/main/bin/brew)
filters out ordinary npm environment variables but preserves `HOMEBREW_*`.
Users can forward an existing registry setting:

```sh
HOMEBREW_NPM_CONFIG_REGISTRY="${NPM_CONFIG_REGISTRY:-}" \
  brew install voidzero-dev/vite-plus/vp
```

Support registry-scoped bearer tokens in `~/.npmrc`, either as literal values or
through `${HOMEBREW_NPM_TOKEN}`. For example, after exporting that variable:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${HOMEBREW_NPM_TOKEN}
```

To reuse a token stored in `NPM_TOKEN`, forward its value as `HOMEBREW_NPM_TOKEN`
and update the `.npmrc` placeholder. Homebrew removes `NPM_TOKEN` from its
environment.

A custom download strategy reads credentials when fetching resources, before
Homebrew's [build sandbox](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/sandbox.rb)
restricts access to `~/.npmrc`. It uses package metadata's `dist.tarball` URL and
scopes credentials to each request's host and path, including redirects.
Credentials must stay out of formula metadata, logs, receipts, and installed
files. Preserve checksums and report authentication errors without falling back
to the public registry. The subsequent pnpm installation runs offline and needs
no credentials.

The registry setting covers CLI packages and their dependencies. Homebrew
itself, its dependencies, and managed Node.js downloads have separate network
requirements. Initial authentication support is limited to bearer tokens.

## Repository layout

```text
vite-plus/
├── HomebrewFormula/
│   ├── vp.rb
│   └── vp/
│       ├── package.json
│       ├── pnpm-lock.yaml
│       └── resources.json
├── lib/homebrew/
│   └── vp_npm_download_strategy.rb
└── .github/
    ├── scripts/update-homebrew-formula.mjs
    └── workflows/                      # release integration and installation tests
```

Support-file names are illustrative. Keep Ruby helpers outside the formula
directory and commit only dependency metadata, not downloaded packages.
Homebrew supports the [`HomebrewFormula/` layout](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#creating-a-tap),
but tapping the repository also downloads unrelated project files. A separate
tap repository would reduce that cost at the expense of a separate review process.

## Commands and migration

Extend the existing [Homebrew detection](../crates/vp_global_cli/src/homebrew.rs)
to identify the installed formula and its tap from the installation path and
receipt. `vp upgrade` must direct users to
`brew upgrade voidzero-dev/vite-plus/vp`, and `vp upgrade --check` to
`brew outdated voidzero-dev/vite-plus/vp`. Explicit versions, force, and rollback
must retain the ownership guard. Keep npm update checks disabled for these
installations, and report the formula identity in `vp env doctor`.

`vp implode` removes user-managed data; `brew uninstall` removes the package.
Document both steps for complete removal. The formula must not delete user data
from an uninstall hook.

Declare a conflict with core's `vite-plus` formula because both install the same
command names. Migration must replace the package and refresh user shims while
preserving preferences, runtimes, and global packages. Do not use `vp implode`
to migrate. Document switching from script installations as well, where existing
shims can take precedence over Homebrew's commands.

Core and script installations remain supported. The tap targets new releases;
backporting it to older versions is outside this proposal.

## Release and launch checks

After all npm packages for a stable release are available, automation opens a
formula update PR. Update the version, dependency lock, target resources, and
checksums together. The formula on `main` must reference a published release;
installation must not resolve `latest` or regenerate the lockfile.

A failed update leaves the previous formula available. Use a formula `revision`
for recipe fixes and a new npm release when package contents must change.
A formula-only merge must not trigger another product release.

Before launch, test:

- Public and authenticated private registries, scoped configuration, redirects,
  and checksum failures, with no credential leaks or unintended public fallback.
- Offline dependency installation and CLI formatting, linting, and builds after
  removing staging files and caches, without a project-local `vite-plus`.
- First and later use with a read-only package and separate user homes; upgrades
  must preserve preferences and working shims after deleting the old version.
- Actual Homebrew fetch, install, reinstall, upgrade, removal, and migration on
  each supported target.

Run these checks for packaging changes and on the `test: install-e2e` label.
Use isolated homes and synthetic credentials; PR tests receive no publishing
credentials. Update user installation and migration guides when the tap is ready.

## Tradeoffs and open decisions

The npm approach reuses existing releases, but requires Node.js and pnpm during
installation, a locked resource list, and a registry downloader. Complete bundles
on GitHub Releases would simplify extraction but add another packaging format.
Keeping only the native binary in Homebrew would require new ownership and
version coordination for JavaScript installed per user.

Before implementation, decide:

1. Who maintains the tap and release automation?
2. Should launch include all four targets, or start with macOS?
3. Is the proposed build-time Node.js and offline pnpm approach acceptable?
4. Are scoped bearer tokens sufficient for the first release?

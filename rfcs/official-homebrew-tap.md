# RFC: Vite+-managed Homebrew tap

- Status: Draft for discussion. This RFC does not create a tap or change installation behavior.
- Original requirement: [Distributing and Installing with Homebrew (#1171)](https://github.com/voidzero-dev/vite-plus/issues/1171).
- Related: [Homebrew first-run failure (#2719)](https://github.com/voidzero-dev/vite-plus/issues/2719), [external installation support (#2729)](https://github.com/voidzero-dev/vite-plus/pull/2729).

## Proposal

Maintain the Homebrew tap in the existing `voidzero-dev/vite-plus` repository.
Add `HomebrewFormula/vp.rb`, with the Ruby class `Vp < Formula`. The tap name is
`voidzero-dev/vite-plus`, and the formula name is `voidzero-dev/vite-plus/vp`.
`vp` is the actual formula name, not an alias for a `vite-plus` formula.

Use the npm registry as the primary distribution source. Download the existing
`@voidzero-dev/vite-plus-cli-<platform>` package for the native binary and install
`vite-plus` at the same version with its production dependencies. Keep the npm
package contents unchanged. No new complete release bundle is required.

Follow the script installer's sequence: acquire the native binary, prepare its
JavaScript dependencies, then configure the user's environment. Homebrew performs
the first two steps in its installation directories. The first `vp` invocation
uses the existing Rust setup code for shell configuration, shims, and preferences.
Homebrew owns the installed binary and JavaScript, including upgrades and removal.

Support `HOMEBREW_NPM_CONFIG_REGISTRY` and registry credentials in `~/.npmrc`
through a tap-owned downloader. Use `HOMEBREW_NPM_TOKEN` in token placeholders
when the value must pass through Homebrew's environment filter.

## Motivation and feasibility

Issue [#1171](https://github.com/voidzero-dev/vite-plus/issues/1171) requested
Homebrew distribution for Bluefin Dx and other Universal Blue / Atomic Fedora
systems. It also identified enterprise management through WorkBrew as a use
case. Issue [#2719](https://github.com/voidzero-dev/vite-plus/issues/2719) later
exposed a first-run setup failure with the core formula on macOS.

The [core formula](https://github.com/Homebrew/homebrew-core/blob/main/Formula/v/vite-plus.rb)
builds from source when a bottle is unavailable or a user requests a source
build. It stages upstream repositories, runs Rust and JavaScript builds, and
deploys the resulting package. A project-owned tap can reuse published npm
packages and avoid those source builds.

The [script installer](../packages/cli/install.sh) already downloads the native
CLI from npm. Its self-setup code installs `vite-plus` through a wrapper manifest
and a pinned pnpm version. The [package publisher](../packages/cli/publish-native-addons.ts)
publishes the native CLI separately from the JavaScript package and native
bindings. These existing packages provide the inputs for the tap.

Current [external setup](../crates/vp_global_cli/src/self_setup.rs) recognizes a
Unix binary with `node_modules/vite-plus` beside its `bin` directory. It reuses
that payload and records setup in the user's state directory. The
[JavaScript resolver](../crates/vp_global_cli/src/js_executor.rs) already supports
this layout. The registry-based approach therefore does not need a new package
format or a new JavaScript resolution layout.

Running `install.sh` unchanged inside a formula would use the installing user's
data directories and configure that user's shell. Installing only its binary
would still trigger standalone bootstrap on first use. The tap must prepare the
JavaScript dependencies in the keg before normal CLI execution.

| Step                    | Script installation                             | Homebrew installation                                                        |
| ----------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------- |
| Version selection       | Requested version or npm tag                    | Exact version pinned by the formula                                          |
| Native CLI              | Existing platform npm tarball                   | The same package, with a formula checksum                                    |
| JavaScript dependencies | pnpm installs into the user's version directory | Pinned pnpm installs into staging; Homebrew installs the result into the keg |
| User setup              | Runs during installer handoff                   | Runs on the user's first `vp` invocation                                     |
| Upgrade and removal     | `vp upgrade` and `vp implode`                   | Homebrew manages the package; `vp implode` manages user data                 |

The current Rust [dependency installer](../crates/vp_setup/src/install.rs)
also provisions managed Node.js and pnpm. The formula should reuse its package
selection and installation layout, with Homebrew supplying the build environment.
It must not call this user-installation routine unchanged. Reading `.npmrc` for
native tarball authentication is also new download logic; the current shell
installer does not provide that functionality.

## Goals and boundaries

- Install existing released npm packages without compiling Vite+ from source.
- Use the same native package, JavaScript version, and pnpm installation model as the script installer.
- Support corporate npm registries and scoped bearer-token authentication.
- Keep package files, upgrades, and removal under Homebrew's control.
- Preserve user settings and working shims across package replacement.
- Keep all published package formats unchanged.

The first version supports stable releases. Proposed targets are macOS ARM64
and x64, and glibc Linux ARM64 and x64. Advertise each target only after its
Homebrew installation tests pass. Windows, musl Linux, preview builds, and
historical formulae remain outside the initial tap scope.

Registry access is required to acquire the CLI and its dependencies unless the
files are already cached. A corporate mirror can provide that access when the
public npm registry is blocked. This design does not provide installation when
both the public registry and all configured mirrors are inaccessible.
Homebrew dependencies, Node.js runtime downloads, and project dependencies have
their own network requirements. The core formula and script installer remain
available independently.

## Installation and ownership

After the tap is available, a new user would run:

```sh
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew install voidzero-dev/vite-plus/vp
vp
```

Use the fully qualified formula name to distinguish it from
`homebrew/core/vite-plus` and core's `vp` alias. The initial `brew tap` command
needs the explicit URL. Otherwise, Homebrew looks for
`voidzero-dev/homebrew-vite-plus`. Subsequent installs and upgrades use the saved
remote. See [the tap command documentation](https://docs.brew.sh/Manpage#tap-options-userrepo-url).

### Repository layout

```text
vite-plus/
├── HomebrewFormula/
│   ├── vp.rb                            # new: formula
│   └── vp/
│       ├── package.json                 # new: pinned installation manifest
│       ├── pnpm-lock.yaml               # new: production dependency lock
│       └── resources.json               # new: package versions and checksums by target
├── lib/homebrew/
│   └── vp_npm_download_strategy.rb      # new: registry configuration and authenticated downloads
├── .github/
│   ├── workflows/
│   │   ├── release.yml                  # extend: open formula update PRs after npm publication
│   │   └── test-homebrew.yml            # new: installation tests
│   └── scripts/
│       └── update-homebrew-formula.mjs   # new: update version, lock, and resource checksums
├── packages/cli/
│   └── install.sh                       # existing script installer
├── crates/vp_global_cli/src/
│   ├── self_setup.rs                    # existing per-user setup
│   └── homebrew.rs                      # ownership and formula identity
└── rfcs/
    └── official-homebrew-tap.md
```

Support-file names are illustrative. Keep Ruby support code outside the formula
directory. The lock and resource manifest are small text files; npm tarballs,
package caches, and installed dependencies are not committed to the repository.

Homebrew recognizes `HomebrewFormula/` at the repository root. Standard tap
installation clones the project repository, including files unrelated to the
formula. It does not limit checkout to this directory. This is the download
cost of keeping packaging in the same repository. See
[Homebrew's tap layout documentation](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#creating-a-tap)
and [clone implementation](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/tap.rb).

The formula on `main` references the last published stable release. Update its
version, dependency lock, and checksums together after the npm packages become
available. Formula PRs follow repository review rules. Use CI path filters to
avoid unrelated product builds for formula-only changes.

### Package preparation

The proposed formula uses the following sequence:

1. Fetch the native CLI and the pinned pnpm package from npm as Homebrew resources.
2. Fetch the locked production dependency tarballs for the target, including required optional native packages.
3. Stage an installation manifest that pins `vite-plus` to the native CLI's version. Point a working copy of the lockfile at the verified local tarballs.
4. Run the pinned pnpm with `--offline --frozen-lockfile --prod --ignore-scripts` in an isolated build directory.
5. Install the native executable into `prefix/bin` and the resulting dependency tree into `prefix/node_modules`. Add `vpr` and `vpx` links to `vp`.

Declare Homebrew Node.js as a build dependency to run pnpm. Pin pnpm to the
version used by the standalone dependency installer, currently `10.33.0`, and
update that selection deliberately. The formula does not depend on a user's
Node.js or pnpm installation. No Rust, CMake, or upstream source checkout is
needed to prepare Vite+ itself.

Generate the wrapper manifest, lock, and resource list during formula-update CI.
Pin transitive versions and integrity values, not only `vite-plus`. The resource
list must match the lock and select the correct OS, CPU, and libc packages.
Use Homebrew's normal resource download stage for authenticated acquisition,
before the build sandbox. Each resource retains a reviewed checksum. The offline
installation then needs neither registry credentials nor access to `~/.npmrc`.
This uses Homebrew's [resource downloads](https://docs.brew.sh/Formula-Cookbook#language-specific-dependencies)
and pnpm's [offline installation mode](https://pnpm.io/cli/install#--offline).

The working lockfile may replace remote tarball locations with staged `file:`
locations while retaining package versions and integrity values. This adapts
Homebrew's verified downloads to pnpm's offline installer. Do not replace locked
integrity values when a mirror serves different bytes.

Use an isolated pnpm store and copy package files into the installed tree.
Internal relative pnpm links are acceptable; no link may depend on the build
directory, an external store, or the user's files. The installed CLI must work
after the build directory and store are removed.

Retain native optional dependencies such as Oxfmt, Oxlint, Rolldown, and the
required platform bindings. Do not use `--no-optional`. The initial design skips
dependency lifecycle scripts and relies on published prebuilt files. Functional
tests must detect any release that starts requiring a build or postinstall
script. Such a release needs a packaging decision before the formula advances.

This preparation follows the script installer's package installation model.
The locked resource list and offline installation are Homebrew-specific additions
that make downloads verifiable and keep credentials outside the build sandbox.
They do not change the published npm packages or require new GitHub bundles.

### Installed layout and first use

```text
<Cellar>/vp/<version>/
├── bin/
│   ├── vp
│   ├── vpr -> vp
│   └── vpx -> vp
├── node_modules/
│   ├── vite-plus/          # may be an internal relative pnpm link
│   └── .pnpm/              # package files and production dependencies
└── INSTALL_RECEIPT.json    # written by Homebrew
```

Keep the real executable at `prefix/bin/vp`. This matches the existing
[Homebrew ownership check](../crates/vp_global_cli/src/homebrew.rs) and JavaScript
resolver. Include applicable license files. The npm package remains named
`vite-plus`; the formula name does not change JavaScript package names or user
directory names.

The formula does not run normal `vp` commands or first-run setup during
installation. It does not modify user shell profiles, set `VP_HOME` or
`VP_*_DIR`, or ship `.vp-setup-complete`. Setup is specific to each user.

On first use, the existing bundled-external setup configures that user's shell,
shims, and preferences. It must not copy the payload, install dependencies, or
write inside the keg. It reports when the user must restart the terminal.
Later invocations reuse the receipt.

Shims follow a stable Homebrew entrypoint. Replacing the package can refresh
its receipt, but must preserve managed mode and mixed per-tool preferences.
Test saved `vp` and tool-shim paths after the previous keg is removed.

Node.js at runtime continues to follow the existing CLI resolver. System-first
mode can use Node.js on `PATH`; managed mode can download a separate runtime.
The Node.js build dependency does not override this user preference or make
all commands work offline. A separate runtime policy would need its own decision.

## Registry configuration and authentication

Use a nonempty `HOMEBREW_NPM_CONFIG_REGISTRY` as the default registry override.
Otherwise, read `registry` from `~/.npmrc`, then fall back to
`https://registry.npmjs.org`. Honor scoped registry settings such as
`@voidzero-dev:registry` for the corresponding packages, as npm does. These scoped
settings take precedence over the default registry. Do not read a project's
`.npmrc` based on the directory from which the user runs `brew`.
The proposed tap implements this setting; it is not a built-in Homebrew setting.

Homebrew's [launcher](https://github.com/Homebrew/brew/blob/main/bin/brew) removes
`NPM_CONFIG_REGISTRY` and ordinary token variables before formula evaluation.
It preserves `HOMEBREW_*` variables. Users could forward an existing setting:

```sh
HOMEBREW_NPM_CONFIG_REGISTRY="${NPM_CONFIG_REGISTRY:-}" \
  brew install voidzero-dev/vite-plus/vp
```

Support registry-scoped `_authToken` entries in `~/.npmrc`. For example, a user
could configure the following and export `HOMEBREW_NPM_TOKEN` in their shell:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${HOMEBREW_NPM_TOKEN}
```

Literal tokens in the file are also supported by this design. Existing entries
that reference `${NPM_TOKEN}` need an explicit forwarding mechanism or a change
to the placeholder; that variable does not survive Homebrew's launcher. Do not
silently substitute a different token or send an unresolved placeholder.

Resolve user configuration and credentials at download time in the tap-owned
download strategy for both the binary and every dependency resource. Do not read
credentials during formula evaluation or from `install`: Homebrew's
[build sandbox](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/sandbox.rb)
restricts access to the user's home, including `.npmrc`. Keep registry tokens
out of formula metadata, receipts, downloaded artifacts, and diagnostic output.
The install step should consume downloaded files without user credentials.

Use the pinned package version's metadata to obtain `dist.tarball`; private
registries can use different tarball paths. Match credentials to each request's
host and path, including tarball requests and redirects. A registry override must
not send another registry's token to the new URL. Retain the formula's SHA-256
check, and report authentication failures without falling back to the public
registry. Follow npm's [configuration and authentication rules](https://docs.npmjs.com/cli/v11/configuring-npm/npmrc/).
The initial authentication support covers bearer tokens. Other enterprise
requirements, such as client certificates, need separate design and validation.

## Commands and diagnostics

Retain the ownership rules introduced for core installations:

| Command                                            | Expected behavior for the tap                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `vp upgrade`                                       | Direct the user to `brew upgrade voidzero-dev/vite-plus/vp`; do not install a managed copy        |
| `vp upgrade --check`                               | Direct the user to `brew outdated voidzero-dev/vite-plus/vp`                                      |
| Upgrade with a version, `--force`, or `--rollback` | Keep the Homebrew ownership guard; do not switch installation channels                            |
| Automatic update check                             | Keep npm checks disabled for Homebrew-owned binaries                                              |
| `vp env doctor`                                    | Report Homebrew ownership, formula identity, binary path, and actual PATH/shim problems           |
| `vp implode`                                       | Explain and remove Vite+-managed user data; retain the keg and direct package removal to Homebrew |
| `brew uninstall voidzero-dev/vite-plus/vp`         | Remove the keg and public links; retain user data                                                 |

Current detection returns a boolean, and command messages use `vite-plus`.
Extend the ownership information to identify the installed formula and read
`source.tap` from the Homebrew receipt when available. Use the full name in
guidance: `voidzero-dev/vite-plus/vp` for this tap and `homebrew/core/vite-plus`
for core. Do not infer the formula name from the npm package name. If an older
or incomplete receipt prevents identification, keep the ownership guard and
ask the user to check the installed formula rather than guess its name.
Do not run `brew` merely to identify the active CLI, and do not add upgrade
advice to unrelated doctor output.

For complete removal, document `vp implode` before `brew uninstall`, including
its removal of runtimes, settings, and global packages. Package removal alone
can leave user shims pointing to an absent entrypoint. Reinstalling the package
or explicitly cleaning those shims resolves that state; the formula must not
delete another user's files from an uninstall hook.

## Release automation and recovery

The release pipeline publishes the existing npm packages. Once the native CLI,
JavaScript package, core package, and required native bindings are available,
a separate job opens a formula update PR in `voidzero-dev/vite-plus`.

The update job selects an exact release version. It generates the installation
manifest, production lock, target resource lists, and SHA-256 checksums from the
published artifacts. Validate package versions and contents before opening the
PR. User installations must not resolve a mutable `latest` tag or regenerate
the dependency lock.

The job only needs write access to this repository. Its credentials must support
required PR checks, through normal events or an explicit workflow dispatch.
Ordinary PR jobs receive no publishing credentials. Use the release version as
an idempotency key, update an existing PR on retry, and reject updates that
replace a newer formula with an older release.

Run installation checks against each candidate before a maintainer merges it.
A formula-only merge must not publish another product release. A failed update
leaves the previous formula available and does not undo npm publication.

Do not overwrite published packages to repair installation. Use a formula
`revision` for a corrected recipe or dependency lock with unchanged product
packages, or publish a new product release when package contents must change.
Keep checksums and dependency metadata consistent. Document recovery for users
already on a bad version: reverting a formula commit alone does not cause
Homebrew to downgrade their installation. `vp upgrade --rollback` remains
unavailable for Homebrew-owned packages.

## Coexistence and migration

The core formula remains independently maintained. Homebrew's
[upstream tap policy](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#upstream-taps)
does not promise removal or redirection of a core formula when an upstream tap
appears. Continue fixing compatibility with core installations.

The tap formula is named `vp`, while the core formula is named `vite-plus`.
They use different Cellar directories, but both install `vp`, `vpr`, and `vpx`
into Homebrew's public `bin`. Declare a conflict with `vite-plus` in the tap
formula and document an explicit migration. Do not overwrite the existing links
or use core's `vp` alias to select the tap package.

The initial documented route should prefetch `voidzero-dev/vite-plus/vp`, remove
`homebrew/core/vite-plus`, and install the fully qualified tap formula. Keep user
data and settings; do not use `vp implode` as a migration step. Check the
installed receipt and active command afterward. Test the exact sequence,
dependency behavior, and recovery from a failed installation before publishing
copy-and-paste instructions.

Switching from a script installation needs separate guidance. Its shims can
precede Homebrew's `bin` on `PATH`. Run the new Homebrew executable by its explicit
path to select it and refresh user shims. Preserve preferences, runtimes, global
packages, and the old payload. Do not silently delete the script installation.
Starting a fresh shell and checking `vp env doctor` must confirm the selected
installation. Switching back must also be tested.

These are migration requirements, not a claim that all existing channel-switching
paths already preserve this state. Resolve any gaps in separate CLI changes
before the tap launch.

Launch only with a release that contains the external setup fixes. There is no
requirement to backport the new tap to old Vite+ releases. Core, script, and
`vp-setup` installations retain their existing behavior.

## Validation before launch

Use isolated homes and synthetic credentials. Test the actual npm tarballs,
including the published native bindings, without a project-local `vite-plus`
installation that could hide missing global dependencies.

The tap tests must cover:

- Native package and JavaScript version agreement, locked dependency checksums, and target filtering.
- Offline dependency installation from downloaded resources with an empty pnpm store and no lifecycle scripts.
- Relocation and operation after the staging directory and pnpm store are removed.
- First and second invocation with a read-only keg; no dependency bootstrap or writes to package files.
- Formatting, linting, and a small production build through the installed global CLI.
- Public registry downloads, a custom registry, scoped registry settings, and `.npmrc` bearer tokens.
- Token substitution, custom `dist.tarball` paths, redirects, authentication failures, missing packages, and checksum failures.
- No credentials in logs, formula metadata, receipts, or installed files; no fallback to the public registry after an authentication error.
- `brew fetch`, installation, reinstall, upgrade, and removal on every supported target.
- Two users sharing the package with separate setup receipts and preferences.
- Removal of an old keg followed by saved `vp` and tool-shim paths in existing Bash and Zsh sessions.
- Mixed management preferences, runtime selection, lifecycle command guidance, and channel migration.

Run Homebrew checks on changes to the formula, downloader, dependency metadata,
and update script. Allow maintainers to request the full installation suite for
other source changes with the `test: install-e2e` label. Run untrusted PR code
without publishing credentials. These checks need not run on every source PR.

## Alternatives

A separate repository such as `voidzero-dev/homebrew-tap` would provide a smaller
checkout and direct installation without an initial custom-URL tap command.
Keeping the formula in `vite-plus` puts code, packaging, and tests in one review
process. A separate repository remains an option if checkout size becomes a
problem.

| Approach                                                             | Benefit                                                                           | Cost or limitation                                                                                                    |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Existing npm packages (proposed)                                     | Reuses released packages and the script installer's dependency installation model | Requires Node.js and pnpm during installation, dependency metadata, and registry authentication support               |
| Complete bundle on GitHub Releases                                   | Simple extraction and no npm access during CLI payload installation               | Requires new bundle assembly, larger assets, and another packaging format                                             |
| Run `install.sh` from a formula                                      | Reuses the entrypoint directly                                                    | Installs user-managed files and loses Homebrew ownership of the active CLI                                            |
| Keep only a native binary in the keg and install JavaScript per user | Avoids dependency installation during `brew install`                              | Requires a new external-binary setup and JavaScript resolution contract, per-user downloads, and version coordination |
| Source formula with project-built bottles                            | Conventional Homebrew packaging                                                   | Retains source recipe complexity and adds a bottle pipeline                                                           |
| Continue with core only                                              | No additional distribution channel                                                | Packaging follows core's process and source build constraints                                                         |

The tap can distribute prebuilt executables inside npm tarballs. This does not
imply that the same recipe would be accepted in `homebrew/core`. The formula
prepares published binaries and packages; `--build-from-source` would not compile
the Vite+ sources. Source builds remain available through the project and core.

## Decisions requested

The primary direction is existing npm packages with Homebrew ownership, using
`HOMEBREW_NPM_CONFIG_REGISTRY` and authenticated `~/.npmrc` support. The npm
package format stays unchanged. Remaining decisions are:

1. Who owns formula updates and Homebrew support within the project?
2. Should the initial tap cover all four proposed targets, or start with macOS while Linux tests mature?
3. Is a Homebrew Node.js build dependency acceptable while the CLI retains its existing runtime selection?
4. Does the locked-resource and offline-pnpm approach provide the right balance of reproducibility and formula complexity?
5. Which enterprise authentication requirements beyond scoped bearer tokens belong in the first version?

After agreement, split implementation into registry downloads, package
installation and formula metadata, CLI formula identity, and release automation
with Homebrew tests. Keep product documentation on the current installation
instructions until the tap is usable. Then update the global CLI, upgrade, and
implode guides, plus the directory layout RFC where the ownership contract
changes.

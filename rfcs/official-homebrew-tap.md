# RFC: Vite+-managed Homebrew tap

- Status: Draft for discussion. This RFC does not create a tap or change installation behavior.
- Original requirement: [Distributing and Installing with Homebrew (#1171)](https://github.com/voidzero-dev/vite-plus/issues/1171).
- Related: [Homebrew first-run failure (#2719)](https://github.com/voidzero-dev/vite-plus/issues/2719), [external installation support (#2729)](https://github.com/voidzero-dev/vite-plus/pull/2729).

## Proposal

Maintain the Homebrew tap in the existing `voidzero-dev/vite-plus` repository.
Add `HomebrewFormula/vp.rb`, with the Ruby class `Vp < Formula`. The tap name is
`voidzero-dev/vite-plus`, and the formula name is `voidzero-dev/vite-plus/vp`.
`vp` is the actual formula name, not an alias for a `vite-plus` formula.

The formula installs a complete, prebuilt CLI bundle from a versioned GitHub
release. Homebrew owns the binary and bundled JavaScript in its Cellar. The
first `vp` invocation uses the existing Rust setup code to configure the user's
shell, shims, and management preferences.

This shares release outputs and setup logic with the script installer. It keeps
package ownership with Homebrew: `brew upgrade` updates the package, and
`brew uninstall` removes it. The formula does not run `install.sh` or install a
second CLI into the user's data directory.

The recommended starting point is a complete bundle, rather than a binary that
downloads its JavaScript on first use. The alternatives below remain open for
discussion.

## Motivation and current behavior

Issue [#1171](https://github.com/voidzero-dev/vite-plus/issues/1171) requested
Homebrew distribution for Bluefin Dx and other Universal Blue / Atomic Fedora
systems. It also identified enterprise management through WorkBrew as a use
case. These requirements motivate Linux support and Homebrew ownership of
installation, upgrades, and removal. Issue
[#2719](https://github.com/voidzero-dev/vite-plus/issues/2719) later exposed a
first-run setup failure with the core formula on macOS.

The [core formula](https://github.com/Homebrew/homebrew-core/blob/main/Formula/v/vite-plus.rb)
builds from source when a bottle is unavailable or a user requests a source
build. It stages upstream repositories, adjusts package-manager configuration,
runs `just build` and `cargo install`, and deploys production JavaScript with
`pnpm`. Normal bottle installs already avoid this build work.

A project-owned tap could reuse the release build instead of maintaining a
second build recipe. It would let the project test and publish packaging changes
with the corresponding CLI release. The project would also take responsibility
for that packaging and its support.

The canonical [script installer](../packages/cli/install.sh) downloads a native
CLI package and delegates installation to that binary's self-setup code. Old
binaries use the legacy installer. This is simpler than duplicating setup in
Ruby, but the standalone path still installs JavaScript dependencies from an
npm registry.

Current [external setup](../crates/vp_global_cli/src/self_setup.rs) already
recognizes a Unix binary with `node_modules/vite-plus` beside its `bin`
directory. It reuses that payload and records setup in the user's state
directory. A tap can use this behavior without taking ownership of the user's
Vite+ directories.

The existing [GitHub release archives](../.github/workflows/reusable-release-build.yml)
contain the native CLI, the version synchronization script, and the toolchain
manifest. They do not contain the complete JavaScript dependency tree. Installing
one of these archives alone would still enter standalone bootstrap. A complete
bundle is new release work, not just a new formula URL.

## Goals and boundaries

- Install the released CLI without a local Rust, CMake, or pnpm build.
- Obtain the complete CLI payload without contacting an npm registry on the user's machine.
- Reuse existing setup, directory resolution, runtime management, and tool dispatch.
- Keep upgrades, package removal, and package files under Homebrew's control.
- Preserve user settings and working shims across package replacement.

The first version supports stable releases. Preview builds, historical formulae,
Windows, and musl Linux remain outside this tap's initial scope. The script
installer retains its existing platforms and version-selection behavior.

This proposal does not remove the core formula or automatically migrate its
users. It also does not make project dependency installation or runtime downloads
work without network access.

## Installation and ownership

After the tap is available, a new user would run:

```sh
brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
brew install voidzero-dev/vite-plus/vp
vp
```

Use the fully qualified formula name in tap instructions to distinguish it from
`homebrew/core/vite-plus` and core's `vp` alias. The initial `brew tap` command
needs the explicit URL. Without it, Homebrew looks for
`voidzero-dev/homebrew-vite-plus`. Subsequent installs and upgrades use the saved
remote. See [the tap command documentation](https://docs.brew.sh/Manpage#tap-options-userrepo-url).

### Repository layout

The proposed files live beside the existing release and installation code:

```text
vite-plus/
├── HomebrewFormula/
│   └── vp.rb                            # new: formula
├── .github/
│   ├── workflows/
│   │   ├── release.yml                  # extend: publish bundles and open formula PRs
│   │   └── test-homebrew.yml            # new: installation tests
│   └── scripts/
│       ├── package-homebrew-bundle.mjs   # new: assemble the release bundle
│       └── update-homebrew-formula.mjs   # new: update version, URLs, and checksums
├── packages/cli/
│   └── install.sh                       # existing script installer
├── crates/vp_global_cli/src/
│   ├── self_setup.rs                    # shared per-user setup
│   └── homebrew.rs                      # ownership and formula identity
└── rfcs/
    └── official-homebrew-tap.md
```

New script and workflow names are illustrative. Only `HomebrewFormula/vp.rb`
is needed for formula discovery. Release bundles remain GitHub release assets;
they are not committed to the source tree.

Homebrew recognizes a root-level `HomebrewFormula/` directory. It taps the Git
repository, not a subdirectory URL. Standard tap installation clones the project
repository, and updates fetch its changes, including changes unrelated to the
formula. It does not limit the checkout to `HomebrewFormula/`. This is the download
cost of keeping packaging in the same repository. See
[Homebrew's tap layout documentation](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#creating-a-tap)
and [clone implementation](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/tap.rb).

The formula on `main` must reference a published stable release, even while the
source contains newer development work. Update it only after the corresponding
bundles and checksums are available. Formula PRs follow the repository's review
rules; CI should use path filters to avoid unnecessary product builds for these
packaging-only changes.

### Installed layout

The formula installs this layout:

```text
<Cellar>/vp/<version>/
├── bin/
│   ├── vp
│   ├── vpr -> vp
│   └── vpx -> vp
├── node_modules/
│   └── vite-plus/          # JS, native bindings, and production dependencies
├── share/                 # completions and license notices
└── INSTALL_RECEIPT.json    # written by Homebrew
```

This layout matches the current [JavaScript resolver](../crates/vp_global_cli/src/js_executor.rs)
and [Homebrew ownership check](../crates/vp_global_cli/src/homebrew.rs).
Keep the real executable at `prefix/bin/vp`; a `libexec` wrapper would require
another ownership and path-resolution contract.
The npm package remains named `vite-plus`; the Homebrew formula name does not
change JavaScript package names or Vite+-managed directory names.

| Owner               | Files and operations                                                                                      |
| ------------------- | --------------------------------------------------------------------------------------------------------- |
| Homebrew            | Cellar payload, public command links, package version, package removal                                    |
| Vite+ for each user | Setup receipt, shell configuration, tool shims, preferences, downloaded runtimes, caches, global packages |

The formula only extracts and installs the bundle. It does not execute `vp`
during `install` or `post_install`, edit shell profiles, or set `VP_HOME` and
`VP_*_DIR`. Completions are generated in release CI and copied as files.
It must not ship `.vp-setup-complete`, because setup is specific to each user.

On first use, the CLI performs the same per-user setup as other bundled external
installations. It must not copy the payload, install its npm dependencies, or
write inside the keg. It reports when the user must restart the terminal to
activate shell changes. Later invocations reuse the receipt.

Shims must follow a stable Homebrew entrypoint, not a versioned Cellar path.
Replacing the package can refresh its receipt, but must preserve both managed
mode and mixed per-tool preferences. Tests must exercise a saved `vp` path and
a tool shim after Homebrew removes the previous keg.

## Bundle and runtime requirements

Add separate assets, tentatively named `vp-bundle-<target>.tar.gz`, to the existing
release. Keep the current small archives unchanged for existing consumers.
Proposed targets are macOS ARM64 and x64, and glibc Linux ARM64 and x64.
Only advertise a target after its Homebrew installation test passes. Minimum OS,
libc, and CPU requirements must match the binaries in the bundle.

Each bundle contains the native CLI and the complete production dependency tree
for the same version and target. Packaging must retain required optional native
packages, including Oxfmt, Oxlint, and Rolldown bindings. Blanket use of
`--no-optional` is unsuitable. Workspace links, pnpm store links, and build-machine
paths must not escape the extracted bundle. Include third-party license notices.

Use the existing release binaries and package outputs. Add an explicit assembly
step for the production dependency tree; the current binary archive step is not
sufficient. The assembly must pin dependency versions, verify package versions,
and test the extracted bundle outside the source checkout. Fetching and preparing
dependencies happens in release CI.

Validate archive contents and dynamic library dependencies for each target.
Missing assets, checksum failures, or an unsupported target must fail installation
without falling back to an npm bootstrap or a source build.

The proposed formula does not depend on Homebrew's `node` and does not bundle a
private Node.js initially. It uses the current runtime resolver: system-first
mode can use Node.js on `PATH`; otherwise the CLI obtains a managed runtime.
Consequently, a first JavaScript command can still download Node.js. Choosing
system-first mode does not guarantee offline operation when no usable Node.js
exists. This runtime choice needs agreement before implementation.

The GitHub bundle proposal removes npm access from CLI payload installation.
It does not remove network access from every command. Users still need access to GitHub release
assets and any required Homebrew downloads. Runtime downloads and project
dependencies have their own network requirements. Existing registry settings
continue to govern npm operations where supported; they do not select the
bundle URL. No new registry variable is needed for GitHub bundles. The npm
download alternative below has separate registry and authentication requirements.

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

The Vite+ release pipeline assembles and tests bundles, then publishes them with
SHA-256 checksums. Formula URLs reference a specific release tag. The formula
contains each target's checksum; it never downloads a mutable `latest` URL or
executes a remote installation script.

After the stable release and required assets are available, a separate job opens
a PR in `voidzero-dev/vite-plus` to update `HomebrewFormula/vp.rb`. The job only
needs write access to this repository. Its credentials must support the required
PR checks, either through normal events or an explicit test workflow dispatch.
Ordinary PR jobs receive no publishing credentials.

The update job verifies every required asset before changing the formula. It
uses the release version as its idempotency key, updates an existing PR on retry,
and rejects an update that would replace a newer formula with an older release.
The Homebrew workflow tests the candidate before a maintainer merges it. A
formula-only merge must not publish another product release. Automatic merging
can be considered after the process proves reliable.

A failed tap update leaves the previous formula available. It does not undo an
already published npm or GitHub release. Report the failure in the release run
and allow the same update job to be retried without rebuilding published assets.

Do not overwrite a published asset to repair packaging. Use an immutable packaging
revision with a new URL and checksum, or a new product release. Document recovery
for users already on a bad version; reverting a formula commit alone does not
cause Homebrew to downgrade those users. The first implementation must not promise
`vp upgrade --rollback` support for Homebrew installations.

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
installed receipt and active command
afterward. Test the exact sequence, dependency behavior, and recovery from a
failed installation before publishing copy-and-paste instructions.

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

Keep the formula installation test small, but cover a real bundled JavaScript
command. A Rust-only `vp --help` test cannot detect missing native dependencies.
Use an isolated user home and explicit management settings.

The release and tap tests must cover:

- Installation without Rust, CMake, or pnpm; no source checkout or external store links in the bundle.
- First and second invocation with a read-only keg, checking that only user setup state changes.
- Formatting, linting, and a small build outside a project-local `vite-plus` installation, to exercise bundled native packages.
- CLI startup and bundled commands with npm registry access blocked, using an available Node.js runtime to isolate that requirement.
- Runtime acquisition with no system Node.js, plus system-first behavior with an existing runtime.
- Two users sharing a package with separate setup receipts and management preferences.
- Package replacement and removal of the old keg, followed by saved `vp` and tool shim paths in existing Bash and Zsh sessions.
- Managed and mixed per-tool settings across upgrades, reinstall, and interrupted setup.
- `upgrade`, `upgrade --check`, `implode`, and doctor output for both formula names, including incomplete receipts.
- Formula discovery through the explicit repository URL, conflict handling, and core's existing `vp` alias.
- Migration in both directions, script installation coexistence, package removal, and retained user data.

Run Homebrew checks on PRs that change `HomebrewFormula/vp.rb` or its packaging
scripts, and bundle checks on release candidates. Allow maintainers to request
the full Homebrew installation suite for other source changes with the
`test: install-e2e` label. Run untrusted PR code without publishing credentials.
These checks need not run on every ordinary source PR.

## Alternatives

A separate repository such as `voidzero-dev/homebrew-tap` would provide a smaller
checkout and direct installation without an initial custom-URL tap command. It
would also need separate repository administration and release-update access.
Keeping the formula in `vite-plus` puts code, packaging, and tests in one review
process. A separate tap remains an option if checkout size becomes a problem;
it is not required for the installation design.

| Approach                                              | Benefit                                                                    | Cost or limitation                                                                                                  |
| ----------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Complete bundle in a project-owned tap (proposed)     | Small formula; no local build or npm bootstrap; Homebrew retains ownership | New bundle assembly, larger assets, and project-owned release support                                               |
| Existing native archive only                          | Small download and nearly identical to script bootstrap                    | Downloads JS on first use; current setup deploys a user-managed copy, so Homebrew no longer controls the active CLI |
| Run `install.sh` from a formula                       | Reuses the installation entrypoint directly                                | Writes user state during package installation and separates installed version tracking from the active CLI          |
| Maintain a source formula and publish our own bottles | Conventional Homebrew build and bottle flow                                | Retains source recipe complexity and adds a bottle build pipeline                                                   |
| Continue with core only                               | No additional formula or distribution channel                              | Packaging updates follow core's process and source build constraints                                                |
| macOS cask                                            | Suitable for a prebuilt macOS distribution                                 | Requires a separate Linux solution and different installation detection                                             |

An upstream tap can distribute prebuilt archives; the
[Bun tap](https://github.com/oven-sh/homebrew-bun/blob/main/Formula/bun.rb)
provides an example. This does not imply that the same formula would be accepted
in core. The proposed binary formula would not offer a source-build mode;
`--build-from-source` would not recreate the upstream build. Source builds remain
available through the project and core.

### npm registry downloads and authentication

The tap could download the existing `@voidzero-dev/vite-plus-cli-<platform>`
package from an npm registry. This option keeps the npm package format unchanged.
It still needs a decision about how to install the JavaScript dependencies under
Homebrew ownership; downloading the native binary alone does not provide them.
Registry authentication must also cover those dependency downloads if this
option is selected.

Use a nonempty `HOMEBREW_VP_NPM_REGISTRY` as a tap-specific default registry override.
Otherwise, read `registry` from `~/.npmrc`, then fall back to
`https://registry.npmjs.org`. Honor scoped registry settings such as
`@voidzero-dev:registry` for the corresponding packages, as npm does. These scoped
settings take precedence over the default registry. Do not read a project's
`.npmrc` based on the directory from which the user runs `brew`.

Homebrew's [launcher](https://github.com/Homebrew/brew/blob/main/bin/brew) removes
`NPM_CONFIG_REGISTRY` and ordinary token variables before formula evaluation.
It preserves `HOMEBREW_*` variables. Users could forward an existing setting:

```sh
HOMEBREW_VP_NPM_REGISTRY="${NPM_CONFIG_REGISTRY:-}" \
  brew install voidzero-dev/vite-plus/vp
```

Support registry-scoped `_authToken` entries in `~/.npmrc`. For example, a user
could configure the following and export `HOMEBREW_VP_NPM_TOKEN` in their shell:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${HOMEBREW_VP_NPM_TOKEN}
```

Literal tokens in the file are also supported by this design. Existing entries
that reference `${NPM_TOKEN}` need an explicit forwarding mechanism or a change
to the placeholder; that variable does not survive Homebrew's launcher. Do not
silently substitute a different token or send an unresolved placeholder.

Resolve user configuration and credentials at download time in a tap-owned
download strategy. Do not read credentials during formula evaluation or from
`install`: Homebrew's [build sandbox](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/sandbox.rb)
restricts access to the user's home, including `.npmrc`. Keep registry tokens
out of formula metadata, receipts, downloaded artifacts, and diagnostic output.
The install step should consume downloaded files without user credentials.

Use the pinned package version's metadata to obtain `dist.tarball`; private
registries can use different tarball paths. Match credentials to each request's
host and path, including tarball requests and redirects. A registry override must
not send another registry's token to the new URL. Retain the formula's SHA-256
check, and report authentication failures without falling back to the public
registry. Follow npm's [configuration and authentication rules](https://docs.npmjs.com/cli/v11/configuring-npm/npmrc/).
This initial option covers bearer tokens; other enterprise requirements such as
client certificates need separate design and validation.

Before selecting this option, use an isolated registry and synthetic `.npmrc`
files to check registry precedence, scoped tokens, token substitution, custom
tarball paths, redirects, authentication failures, checksum failures, and secret
redaction. Test `brew fetch`, installation, reinstall, and upgrade on macOS and
Linux. This is a proposed download design, not existing tap functionality.

## Decisions requested

1. Should Homebrew retain ownership of the active CLI, with complete bundles as proposed, or should the tap only bootstrap a script-managed install?
2. Who owns formula updates and Homebrew support within the Vite+ project?
3. Should the initial tap cover all four proposed targets, or start with macOS while Linux tests mature?
4. Is the existing Node.js runtime resolver sufficient, or should the tap require Homebrew Node.js or ship a private runtime?
5. Should tap updates require maintainer review initially, and what packaging-repair version scheme should we use?
6. Should the tap use GitHub bundles, or existing npm packages with `HOMEBREW_VP_NPM_REGISTRY` and authenticated `~/.npmrc` support? The npm option also needs a dependency installation design.

After agreement, split implementation into bundle assembly, CLI ownership
metadata, the tap and its tests, and release publication. Keep product
documentation on the current installation instructions until the tap is usable.
Then update the global CLI, upgrade, and implode guides, plus the directory
layout RFC where the ownership metadata contract changes.

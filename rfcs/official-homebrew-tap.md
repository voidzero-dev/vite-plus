# RFC: Vite+-managed Homebrew tap

- Status: Draft for discussion. This RFC does not create a tap or change installation behavior.
- Related: [Homebrew first-run failure (#2719)](https://github.com/voidzero-dev/vite-plus/issues/2719), [external installation support (#2729)](https://github.com/voidzero-dev/vite-plus/pull/2729).

## Proposal

Maintain a public Homebrew tap under the VoidZero organization. The proposed
repository is `voidzero-dev/homebrew-tap`, with the formula
`voidzero-dev/tap/vite-plus`.

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
brew install voidzero-dev/tap/vite-plus
vp
```

Use the fully qualified formula name in tap instructions to distinguish it from
`homebrew/core/vite-plus`. Homebrew can add a tap during direct installation.
See [Homebrew's tap guide](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#installing).

The formula installs this layout:

```text
<Cellar>/vite-plus/<version>/
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

The tap removes npm access from CLI payload installation. It does not remove
network access from every command. Users still need access to GitHub release
assets and any required Homebrew downloads. Runtime downloads and project
dependencies have their own network requirements. Existing registry settings
continue to govern npm operations where supported; they do not select the
bundle URL. No new registry variable is needed for the proposed formula.

## Commands and diagnostics

Retain the ownership rules introduced for core installations:

| Command                                            | Expected behavior for the tap                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `vp upgrade`                                       | Direct the user to `brew upgrade voidzero-dev/tap/vite-plus`; do not install a managed copy       |
| `vp upgrade --check`                               | Direct the user to `brew outdated voidzero-dev/tap/vite-plus`                                     |
| Upgrade with a version, `--force`, or `--rollback` | Keep the Homebrew ownership guard; do not switch installation channels                            |
| Automatic update check                             | Keep npm checks disabled for Homebrew-owned binaries                                              |
| `vp env doctor`                                    | Report Homebrew ownership, formula identity, binary path, and actual PATH/shim problems           |
| `vp implode`                                       | Explain and remove Vite+-managed user data; retain the keg and direct package removal to Homebrew |
| `brew uninstall voidzero-dev/tap/vite-plus`        | Remove the keg and public links; retain user data                                                 |

Current detection returns a boolean, and command messages use `vite-plus`.
Extend the ownership information to obtain the tap from the Homebrew receipt
when available. Use the formula's full name in guidance. Preserve the existing
generic guidance for older or incomplete receipts. Do not run `brew` merely to
identify the active CLI, and do not add upgrade advice to unrelated doctor output.

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
an update PR in the tap. A narrowly scoped GitHub App token grants write access
to that repository. Ordinary PR jobs receive no publishing credentials.

The update job verifies every required asset before changing the formula. It
uses the release version as its idempotency key, updates an existing PR on retry,
and rejects an update that would replace a newer formula with an older release.
Tap CI tests the candidate before a maintainer merges it. Automatic merging can
be considered after the process proves reliable.

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

The tap and core formula share the name `vite-plus` and cannot be installed
side by side in the same Homebrew prefix. Migration is explicit. The initial
documented route should prefetch the tap package, remove the core package, and
install the fully qualified tap formula. Keep user data and settings; do not use
`vp implode` as a migration step. Check the installed receipt and active command
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
- `upgrade`, `upgrade --check`, `implode`, and doctor output for both core and tap receipts.
- Migration in both directions, script installation coexistence, package removal, and retained user data.

Run tap checks on formula PRs and bundle checks on release candidates. In the
Vite+ repository, allow maintainers to request the full Homebrew installation
suite with the `test: install-e2e` label. Run untrusted PR code without publishing
credentials. These checks need not run on every ordinary source PR.

## Alternatives

| Approach                                              | Benefit                                                                    | Cost or limitation                                                                                                  |
| ----------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Complete bundle in a project-owned tap (proposed)     | Small formula; no local build or npm bootstrap; Homebrew retains ownership | New bundle assembly, larger assets, and project-owned release support                                               |
| Existing native archive only                          | Small download and nearly identical to script bootstrap                    | Downloads JS on first use; current setup deploys a user-managed copy, so Homebrew no longer controls the active CLI |
| Run `install.sh` from a formula                       | Reuses the installation entrypoint directly                                | Writes user state during package installation and separates installed version tracking from the active CLI          |
| Maintain a source formula and publish our own bottles | Conventional Homebrew build and bottle flow                                | Retains source recipe complexity and adds a bottle build pipeline                                                   |
| Continue with core only                               | No new repository or distribution channel                                  | Packaging updates follow core's process and source build constraints                                                |
| macOS cask                                            | Suitable for a prebuilt macOS distribution                                 | Requires a separate Linux solution and different installation detection                                             |

An upstream tap can distribute prebuilt archives; the
[Bun tap](https://github.com/oven-sh/homebrew-bun/blob/main/Formula/bun.rb)
provides an example. This does not imply that the same formula would be accepted
in core. The proposed binary formula would not offer a source-build mode;
`--build-from-source` would not recreate the upstream build. Source builds remain
available through the project and core.

## Decisions requested

1. Should Homebrew retain ownership of the active CLI, with complete bundles as proposed, or should the tap only bootstrap a script-managed install?
2. Is `voidzero-dev/homebrew-tap` the right repository name, and who owns release updates and support?
3. Should the initial tap cover all four proposed targets, or start with macOS while Linux tests mature?
4. Is the existing Node.js runtime resolver sufficient, or should the tap require Homebrew Node.js or ship a private runtime?
5. Should tap updates require maintainer review initially, and what packaging-repair version scheme should we use?

After agreement, split implementation into bundle assembly, CLI ownership
metadata, the tap and its tests, and release publication. Keep product
documentation on the current installation instructions until the tap is usable.
Then update the global CLI, upgrade, and implode guides, plus the directory
layout RFC where the ownership metadata contract changes.

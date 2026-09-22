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

A project-maintained tap can install the native CLI from GitHub Releases and
reuse the script installer's first-run dependency setup. This keeps npm access
out of `brew install` and lets setup use the user's npm configuration.

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

Homebrew downloads the target's [GitHub release archive](https://github.com/voidzero-dev/vite-plus/releases),
verifies its pinned SHA-256 checksum, and installs `vp` with `vpr` and `vpx`
aliases. The formula needs no npm access, Node.js, or pnpm during installation.

On first use, `vp` installs the matching `vite-plus` npm package and its
production dependencies into the user's data directory. It then configures the
user's settings and shims. Homebrew retains ownership of the executable; Vite+
manages each user's dependencies. Published package formats stay unchanged.

The initial scope is stable releases on macOS and glibc Linux, for ARM64 and
x64. The [release build](../.github/workflows/reusable-release-build.yml)
already produces archives for all four targets. Launch each target after its
Homebrew installation tests pass. Treat Intel macOS as best-effort, consistent
with [Homebrew's support tiers](https://docs.brew.sh/Support-Tiers).

Installation assumes network access: GitHub for the tap and binary, and the
configured npm registry and runtime sources for first-run setup.

## Installation flow

The snapshots show a fresh installation, then an upgrade from version `A` to
`B`. `<BREW>` is Homebrew's prefix, such as `/opt/homebrew`. User directories
`<DATA>`, `<BIN>`, `<CONFIG>`, and `<STATE>` come from `VpDirs` and respect the
user's directory overrides. Dependency paths and receipt names are illustrative.
Each stage shows relevant additions or changes; other files are omitted.

### 1. Register the tap

`brew tap` clones the repository. At this point, Homebrew has the formula but
has not installed the CLI.

```text
<Homebrew tap checkout>/
└── HomebrewFormula/vp.rb
```

### 2. Install the native CLI

`brew install` downloads the GitHub archive, installs the CLI into the Cellar,
and links the public commands. It creates no per-user Vite+ installation.

```text
<BREW>/
├── Cellar/vp/A/
│   ├── bin/
│   │   ├── vp
│   │   ├── vpr -> vp
│   │   └── vpx -> vp
│   └── INSTALL_RECEIPT.json
└── bin/
    ├── vp  -> ../Cellar/vp/A/bin/vp
    ├── vpr -> ../Cellar/vp/A/bin/vpr
    └── vpx -> ../Cellar/vp/A/bin/vpx
```

### 3. Prepare dependencies on first use

The first `vp` invocation prepares Node.js and uses its bundled npm to download
the pinned pnpm package. Then pnpm installs `vite-plus@A` and production
dependencies in the user's data directory, keyed by CLI version and platform.
The Homebrew files stay unchanged.

```text
<DATA>/
├── js_runtime/node/<node-version>/
│   ├── bin/node
│   └── lib/node_modules/npm/        # included in the Node.js distribution
├── package_manager/pnpm/<pnpm-version>/pnpm/
└── cli-packages/A/<platform>/
    ├── package.json
    ├── pnpm-lock.yaml
    └── node_modules/
        ├── vite-plus/
        └── ...                     # production dependencies
```

### 4. Complete user setup

Setup creates preferences, shell environment files, and shims. It records
completion in user-owned state only after installation succeeds. All shims
link through Homebrew's stable public command. This example uses managed
Node.js and system-first npm:

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

The executable remains at `<BREW>/Cellar/vp/A/bin/vp` and loads JavaScript from
`<DATA>/cli-packages/A/<platform>/node_modules/vite-plus`. Later invocations
reuse these files. The generated environment follows the existing
[directory layout](directory-layout.md): `<BIN>` comes first on `PATH`, and
`<DATA>/fallback-bin` comes last. System tools take precedence over fallback
shims. These preferences are independent of the tools used to install dependencies.

### 5. Upgrade through Homebrew

`brew upgrade` installs version `B` and replaces its public links. User shims
continue to point through those links, even after Homebrew removes version `A`.

```text
<BREW>/Cellar/vp/B/
├── bin/                            # new vp, vpr, vpx
└── INSTALL_RECEIPT.json

<BREW>/bin/
├── vp  -> ../Cellar/vp/B/bin/vp
├── vpr -> ../Cellar/vp/B/bin/vpr
└── vpx -> ../Cellar/vp/B/bin/vpx
```

The first invocation of `B` adds its matching dependencies and completion record.
It preserves the user's preferences and reuses existing bootstrap tools when
applicable.

```text
<DATA>/cli-packages/B/<platform>/
├── package.json
├── pnpm-lock.yaml
└── node_modules/
    ├── vite-plus/
    └── ...

<STATE>/self-setup/
└── <receipt-for-B>.json
```

Dependency setup runs once per user, version, and platform. Serialize concurrent
attempts and allow interrupted installs to retry without repeating preference
prompts. Keep these directories separate from the script installer's `current`
link.

## Required CLI changes

Add a mode for Homebrew-owned binaries without bundled JavaScript. The current
[self-setup code](../crates/vp_global_cli/src/self_setup.rs) copies bare external
binaries into a managed installation; this mode must retain the Homebrew binary.
The [JavaScript resolver](../crates/vp_global_cli/src/js_executor.rs) must find
its matching per-user dependency directory instead of expecting JavaScript
beside the executable.

Reuse [environment setup](../crates/vp_global_cli/src/commands/env/setup.rs)
to initialize missing package-manager preferences and place shims. Preserve
saved choices when dependencies change. Both shim directories must retain
stable Homebrew targets across upgrades.

Existing script installations and core installations with bundled JavaScript
must retain their behavior. Automatic setup needs no new public `vp setup`
command. Setup must not write into the Cellar.

## Registry configuration

The user runs `vp` outside Homebrew's build sandbox. Both npm and pnpm read
`~/.npmrc`, or the file selected by `NPM_CONFIG_USERCONFIG`, and inherit registry
and token environment variables. Use [npm-compatible authentication](https://docs.npmjs.com/cli/v11/configuring-npm/npmrc/)
at both stages, including registry-scoped tokens and basic authentication:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${NPM_TOKEN}
```

Users export `NPM_TOKEN` before running `vp`. They can also use
`NPM_CONFIG_REGISTRY` for a registry override. No Homebrew-specific npm variables
or Homebrew npm downloader are required.

Change the shared [dependency installer](../crates/vp_setup/src/install.rs) to
fetch pinned pnpm with Node's bundled npm. Invoke Node and `npm-cli.js` by their
absolute paths. Use [`npm pack`](https://docs.npmjs.com/cli/v11/commands/npm-pack/)
with an exact version, `--ignore-scripts`, `--json`, and `--workspaces=false` in
a temporary directory. npm handles registry metadata, tarball URLs, integrity,
and credentials. Reuse the existing pnpm cache layout and installation lock.
Then use that pnpm for dependency installation.

Run both stages outside the calling project's workspace. Resolve an explicit
relative `NPM_CONFIG_USERCONFIG` before changing directories, and pass the same
registry override to both stages. Keep credentials out of command arguments,
logs, and receipts. Authentication failures must stop setup without public
registry fallback. pnpm-only credential helpers are outside the initial scope.

## Commands and migration

Extend [Homebrew detection](../crates/vp_global_cli/src/homebrew.rs) to identify
the installed formula and tap. `vp upgrade` directs users to
`brew upgrade voidzero-dev/vite-plus/vp`; `vp upgrade --check` directs them to
`brew outdated voidzero-dev/vite-plus/vp`. Keep the ownership guard for explicit
versions, force, and rollback, and keep npm update checks disabled for the CLI.
`vp env doctor` reports the Homebrew executable and per-user dependency location.

`vp implode` removes user-managed dependencies, runtimes, settings, and shims.
`brew uninstall` removes the executable. Document both steps for complete
removal; the formula must not delete user data from an uninstall hook.

Declare a conflict with core's `vite-plus` formula because both install the same
command names. Migration must refresh shims while preserving preferences,
runtimes, and global packages. Do not use `vp implode` to migrate. Document
switching from script installations, whose shims may take precedence on `PATH`.
Core and script installations remain supported; the tap targets new releases.

## Repository and releases

```text
vite-plus/
├── HomebrewFormula/vp.rb
└── .github/
    ├── scripts/update-homebrew-formula.mjs
    └── workflows/                      # release integration and installation tests
```

Homebrew supports the [`HomebrewFormula/` layout](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap#creating-a-tap),
but tapping this repository also downloads unrelated project files. A separate
tap repository would reduce that cost at the expense of a separate review process.

After both the GitHub archives and matching npm packages are published,
automation opens a formula update PR with the version, target URLs, and
checksums. A failed update leaves the previous formula available. Use a formula
`revision` for recipe fixes and a new release for CLI changes. A formula-only
merge must not trigger another product release.

Vite+ release maintainers own the formula and its release automation through
the existing repository review process. Confirm a primary maintainer and a
backup before launch. Formula failures belong in this repository's issue tracker.

## Launch checks

Test with isolated homes and synthetic credentials:

- `brew install` downloads the release and creates the commands without
  preinstalled Node.js or pnpm. The formula does not run npm or user setup.
- First use on a fresh home bootstraps dependencies through public and
  authenticated private registries, including pnpm's own download. Cover token
  and basic authentication, registry and user-config overrides, and rejection
  of invalid credentials or corrupted tarballs.
- Later invocations reuse dependencies; failed and concurrent setup attempts
  recover without repeated preference prompts or writes to the Cellar.
- Formatting, linting, and builds use the matching package without a
  project-local `vite-plus` masking missing dependencies.
- Homebrew upgrade, removal of the old version, separate users, uninstall,
  `vp implode`, and migration preserve the ownership rules above.
- Mixed management preferences survive upgrades. Commands through both shim
  directories, including `pn` and `pnx`, still work after the old Cellar version
  is removed.

Run installation checks for packaging changes and on the `test: install-e2e`
label. PR tests receive no publishing credentials. Update installation and
migration guides when the tap is ready.

## Tradeoffs

The formula stays small and dependency installation shares the script installer's
code. Each user downloads and stores their own dependencies. Homebrew manages
the executable; Vite+ manages the per-user JavaScript installation.

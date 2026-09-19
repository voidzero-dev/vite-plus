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
x64. Launch each target only after its installation tests pass.

## First-run setup

Add a setup mode for a Homebrew-owned binary without bundled JavaScript:

1. Keep the executable in Homebrew's Cellar. Reuse the shared Node.js and pinned
   pnpm bootstrap, then install `vite-plus` at the executable's exact version.
2. Store dependencies in a dedicated directory under `VpDirs.data`, keyed by
   CLI version and platform, separate from the script installer's `current` link.
3. Record successful dependency installation in user-owned state. Reuse existing
   per-user setup for shims and management preferences.
4. Execute commands through the Homebrew binary using the matching dependency
   directory. Later invocations reuse that installation.

For example:

```text
Homebrew: <Cellar>/vp/<version>/bin/vp
User:     <DATA>/cli-packages/<version>/<platform>/node_modules/vite-plus
```

The user path is illustrative; resolve it through `VpDirs`. Shims must use a
stable Homebrew entrypoint and keep working after the old Cellar version is
removed. Setup must not copy the executable or write into the Cellar.

After a Homebrew upgrade, the new binary prepares its dependencies on first use.
This is once per user, version, and platform, not once for the machine. Preserve
saved management choices. Serialize concurrent setup attempts and record
completion only after a successful install, so interrupted downloads can retry.

The current [self-setup code](../crates/vp_global_cli/src/self_setup.rs) copies
bare external binaries into a managed installation. The
[JavaScript resolver](../crates/vp_global_cli/src/js_executor.rs) expects
JavaScript beside the executable. Both need changes for this mode. Existing
script installations and core installations with bundled JavaScript must retain
their behavior. Automatic setup needs no new public `vp setup` command.

Runtime Node.js selection keeps the existing system-first and managed modes.
Bootstrapping the tools needed for dependency installation must not change the
user's saved runtime preferences.

## Registry configuration

The user runs `vp` outside Homebrew's build sandbox, so pnpm can read `~/.npmrc`
and inherit normal registry and token environment variables. Follow
[pnpm's registry and authentication rules](https://pnpm.io/10.x/npmrc), including
scoped registries and user-level token placeholders:

```ini
registry=https://registry.example.com/repository/npm/
//registry.example.com/repository/npm/:_authToken=${NPM_TOKEN}
```

Users export `NPM_TOKEN` before running `vp`. They can also use
`NPM_CONFIG_REGISTRY` for a registry override. No Homebrew-specific npm variables
or Homebrew npm downloader are required. Run dependency installation in its own
directory so the calling project's workspace and `.npmrc` do not control it.

One bootstrap gap remains: the shared [dependency installer](../crates/vp_setup/src/install.rs)
downloads pnpm through Rust before pnpm can read configuration. That download
must honor the user's registry and credentials too. Fix and test this shared
bootstrap path before claiming support for authenticated private registries.
Keep credentials out of logs and receipts, and do not fall back to the public
registry after authentication fails.

First use requires access to the configured npm registry and any uncached
Node.js/bootstrap downloads. GitHub access alone is sufficient for the tap's
package download, but does not make the full CLI ready for offline use.

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

## Launch checks

Test with isolated homes and synthetic credentials:

- `brew install` succeeds with npm access blocked and no preinstalled Node.js or
  pnpm. The formula does not trigger user setup.
- First use on a fresh home bootstraps dependencies through public and
  authenticated private registries, including pnpm's own download.
- Later invocations reuse dependencies; failed and concurrent setup attempts
  recover without repeated preference prompts or writes to the Cellar.
- Formatting, linting, and builds use the matching package without a
  project-local `vite-plus` masking missing dependencies.
- Homebrew upgrade, removal of the old version, separate users, uninstall,
  `vp implode`, and migration preserve the ownership rules above.

Run installation checks for packaging changes and on the `test: install-e2e`
label. PR tests receive no publishing credentials. Update installation and
migration guides when the tap is ready.

## Tradeoffs and open decisions

The formula stays small and dependency installation shares the script installer's
code. Each user downloads dependencies, and the first invocation after an upgrade
may need network access. Homebrew manages the executable rather than the complete
JavaScript installation.

Installing dependencies into the Cellar would make them available to all users,
but requires npm configuration handling during Homebrew installation. A complete
GitHub bundle would avoid that step at the cost of another packaging format.

Before implementation, decide:

1. Who maintains the tap and release automation?
2. Should launch include all four targets, or start with macOS?
3. How should the shared pnpm bootstrap reuse npm authentication configuration?

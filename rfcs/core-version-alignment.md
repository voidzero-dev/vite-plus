# RFC: Align the Core Package Version with the Bundled Vite Version

- Status: Proposed
- Issue: [#2806](https://github.com/voidzero-dev/vite-plus/issues/2806)
- Related: [Q4 2026 plan](https://github.com/voidzero-dev/vite-plus/issues/3277),
  [Vite core module identity](./vite-core-module-identity.md),
  [Core binding resolution](./core-binding-resolution.md),
  [Migration rules](../docs/guide/migrate-rules.md),
  [Core package bundling](../packages/core/BUNDLING.md)

## Proposal

Derive the version of `@voidzero-dev/vite-plus-core` from the Vite version it
bundles. The core major and minor equal the bundled Vite major and minor. The
core patch encodes the bundled Vite patch and a release counter:

```text
stable Vite+ release:          <vite major>.<vite minor>.<vite patch * 1000 + n>
Vite+ prerelease, stable Vite: <next stable core version>-vp.<k>
Vite+ prerelease, Vite pre:    <vite version>.vp.<k>
```

`n` counts the stable Vite+ releases that bundle the same Vite patch, from 0
to 998. `n = 999` is reserved for local and CI builds. `k` counts the Vite+
prereleases on the same base, from 0. Decoding is exact: the bundled Vite
patch is `floor(core patch / 1000)`.

This is the convention of `oxlint-tsgolint`, which Vite+ bundles. Its first
release on TypeScript `7.0.2` is `7.0.2000`, and the next ones are `7.0.2001`,
`7.0.2002`, and so on
([tsgolint versioning](https://github.com/oxc-project/tsgolint#versioning)).
One rule then applies to the versions that `vp --version` shows.

| Vite+ release     | Bundled Vite   | Core version        |
| ----------------- | -------------- | ------------------- |
| `1.5.0` (first)   | `8.3.5`        | `8.3.5000`          |
| `1.5.1`           | `8.3.5`        | `8.3.5001`          |
| `1.6.0`           | `8.3.6`        | `8.3.6000`          |
| `1.7.0-rc.0`      | `8.4.0`        | `8.4.0-vp.0`        |
| `1.7.0-rc.1`      | `8.4.0`        | `8.4.0-vp.1`        |
| `1.7.0`           | `8.4.0`        | `8.4.0`             |
| `1.7.1`           | `8.4.1`        | `8.4.1000`          |
| `2.0.0-beta.0`    | `9.0.0-beta.3` | `9.0.0-beta.3.vp.0` |
| `2.0.0-beta.1`    | `9.0.0-beta.3` | `9.0.0-beta.3.vp.1` |
| Local or CI build | `8.3.5`        | `8.3.5999`          |

`vite-plus` keeps its own version line (`1.x` today). The CLI keeps its exact
alias dependency `vite: npm:@voidzero-dev/vite-plus-core@<core version>`.
Projects keep the same alias in their manifests, overrides, catalogs, and
resolutions. `vp migrate` stops adding `vite` to pnpm `peerDependencyRules`
and removes the entry that earlier migrations wrote.

## Before and after

For a pnpm project, `vp migrate` removes the broad `vite` peer rule from
`pnpm-workspace.yaml`. Plugins that support the bundled Vite pass the peer
check without it, and incompatible plugins are reported again. Example: an
upgrade from `vite-plus` `1.5.1` to a release that bundles Vite `8.3.7`:

```diff
 overrides:
   vite@*: 'catalog:'
-peerDependencyRules:
-  allowAny:
-    - vite
-  allowedVersions:
-    vite: '*'
 catalog:
-  vite: npm:@voidzero-dev/vite-plus-core@1.5.1
+  vite: npm:@voidzero-dev/vite-plus-core@8.3.7000
-  vite-plus: 1.5.1
+  vite-plus: 1.6.0
```

The rule stays on pnpm before 10.2.0, and for preview and prerelease cores
(see Migration).

## Problem

A migrated project aliases `vite` to the core package:

```json
{
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@1.4.0",
    "vite-plus": "1.4.0"
  }
}
```

Package managers resolve the alias and read the target's manifest. They
compare its `version` (`1.4.0`) with each consumer's `vite` peer range. Every
current plugin range fails: `vitest@5.0.5` declares
`^6.4.0 || ^7.0.0 || ^8.0.0`, `@vitejs/plugin-react@6.1.2` declares `^8.0.0`,
`@tanstack/react-start` declares `>=8.0.0`.

pnpm users need `peerDependencyRules.allowAny: [vite]` and
`allowedVersions: { vite: '*' }`. `vp migrate` writes these rules in
`packages/cli/src/migration/migrator/vite-plus-bootstrap.ts`
(`ensurePnpmPeerDependencyRules`), `catalog.ts` (`rewritePnpmWorkspaceYaml`),
and `orchestrators.ts`. The rules also hide real incompatibilities: a plugin
that declares `vite: ^7.0.0` installs without a warning.

npm has no per-package suppression. The `vite-plus` package alone is
inconsistent from npm's point of view: it depends on
`vite: npm:@voidzero-dev/vite-plus-core@1.4.0` and on `vitest`, whose peer
range rejects `1.4.0`. Every `npm install` prints
`ERESOLVE overriding peer dependency` warnings, and `npm ls vite` marks the
alias `invalid`. The only workaround is `legacy-peer-deps=true`, which changes
peer resolution for the whole tree.

The release pipeline sets one version on every package.
`.github/workflows/prepare_release.yml` runs `update_json` on
`packages/cli/package.json` and `packages/core/package.json` with the same
`VERSION`. `.github/workflows/release.yml` publishes
`@voidzero-dev/vite-plus-core@${VERSION}`, and
`.github/workflows/reusable-release-build.yml` stamps `${VERSION}` into both
manifests before the build.

## Goals and scope

For a project whose bundled Vite satisfies a consumer's `vite` peer range,
the package manager must report the peer as satisfied. For a consumer whose
range excludes the bundled Vite major or minor, the package manager must still
report it. Patch-level floors must resolve correctly up to the bundled patch.

Migration must stop writing the broad `vite` peer rule and must remove the one
it wrote earlier, without touching rules that users added for other packages.

The core version must be derivable from the bundled Vite version, and the
bundled Vite version from the core version, without a lookup table. CI must
check that the committed core version encodes `bundledVersions.vite`.

Out of scope:

- The `vitest` entries in the same peer rules. `vitest` is pinned to a real
  upstream version (`VITEST_VERSION` in `packages/cli/src/utils/constants.ts`),
  so peer checks against it are already accurate. The `vitest` rule hides
  real incompatibilities in the same way; removing it is a separate change.
- The `vite-plus` version policy. This RFC does not decide whether a Vite
  major upgrade is a `vite-plus` major release.
- The registry bridge for preview builds. It assigns `0.0.0-commit.<sha>` to
  every workspace package and is a separate repository.

## Design

### Version scheme

The scheme must make four range shapes resolve like the bundled Vite:
`^MAJOR.0.0`, `^MAJOR.MINOR.0`, `~MAJOR.MINOR.0`, and `>=MAJOR.MINOR.PATCH`.
Three options were compared against bundled Vite `8.3.5` with `vite-plus`
`1.4.0`:

- Option A: core major = Vite major; minor and patch follow `vite-plus`
  (`8.4.0`).
- Option B: core major and minor = Vite major and minor; patch is a plain
  counter per Vite+ release (`8.3.0`, `8.3.1`, ...).
- Option B+ (selected): like B, but the patch encodes the Vite patch
  (`8.3.5000`, `8.3.5001`, ...).
- Option C: core = exact Vite version plus a prerelease tag
  (`8.3.5-vp.1.4.0`) or build metadata (`8.3.5+vp.1.4.0`).

| Consumer range | Bundled `8.3.5` | A `8.4.0`    | B `8.3.0`    | B+ `8.3.5000` | C `8.3.5-vp.1.4.0` (npm) |
| -------------- | --------------- | ------------ | ------------ | ------------- | ------------------------ |
| `^8.0.0`       | pass            | pass         | pass         | pass          | fail (wrong)             |
| `^8.3.0`       | pass            | pass         | pass         | pass          | fail (wrong)             |
| `>=8.3.2`      | pass            | pass         | fail (wrong) | pass          | fail (wrong)             |
| `~8.3.0`       | pass            | fail (wrong) | pass         | pass          | fail (wrong)             |
| `^7.0.0`       | fail            | fail         | fail         | fail          | fail                     |
| `^8.4.0`       | fail            | pass (wrong) | fail         | fail          | fail                     |
| `>=8.3.6`      | fail            | pass (wrong) | fail         | pass (wrong)  | fail                     |

Option A fails on minor ranges in both directions. The `vite-plus` minor and
the Vite minor drift apart: `v1.2.0` bundled Vite `8.3.4`, so core `8.2.0`
would reject `^8.3.0` consumers, and a later `vite-plus` `1.9.x` on Vite
`8.5.x` would accept `>=8.6.0` consumers. Both are the failure the RFC exists
to remove.

Option B resolves minor ranges correctly. Its patch carries no information, so
a `>=8.3.2` consumer fails against `8.3.0` or `8.3.1` although the bundled
`8.3.5` satisfies it. A consumer `>=8.3.6` passes when the counter reaches
the value 6. Patch floors are less common than minor floors, but they exist,
and the counter value is unrelated to the real patch in both directions.

Option C fails the main goal. npm's semver compares a prerelease only against
comparators with the same `major.minor.patch` tuple, so `8.3.5-vp.1.4.0`
fails `^8.0.0`, `>=8.0.0`, and `^8.3.0`. npm users keep every warning from
#2806. The experiment below confirms this behavior for npm and shows that pnpm
and Yarn are more lenient. Build metadata does not help: npm treats
`8.3.5+a` and `8.3.5+b` as the same version, so a second Vite+ release on the
same Vite patch cannot publish. `v1.2.0` and `v1.3.0` both bundled Vite
`8.3.4`, so that case is real.

Option B+ is the selected scheme. Its residual false results are limited to
patch floors above the bundled patch inside the same minor (`>=8.3.6` passes
against `8.3.5000`) and to exact pins (`8.3.5` fails against `8.3.5000`). Both
are rare in `vite` peer ranges. Vite+ follows Vite patch releases within days,
so the window for the first case is short.

The counter supports 999 stable Vite+ releases per Vite patch (`n` from 0 to
998). `n = 999` is reserved for local and CI builds (see Local and CI builds).
Vite+ shipped five minor releases between 2026-09-28 and 2026-10-23 while Vite
moved from `8.3.1` to `8.3.5`. Two releases on one Vite patch happened once
(`v1.2.0` and `v1.3.0` on `8.3.4`).

Core versions keep sorting correctly:
`8.3.5001 < 8.3.5002-vp.0 < 8.3.5002 < 8.3.6000`.

### Prereleases and new Vite majors

A Vite+ prerelease does not copy its own prerelease identifiers into the core
version. A copied suffix has two problems:

- Two Vite+ prereleases can have the same identifiers, for example
  `1.7.0-rc.0` and `1.8.0-rc.0`. If no stable release ships between them on
  the same Vite, both map to `8.4.0-rc.0`, and the second publish fails.
- On a stable Vite, a copied suffix looks like a Vite prerelease and can sort
  out of order. A Vite+ RC on Vite `9.0.0` would publish `9.0.0-rc.0`, which
  sorts below a core built on Vite `9.0.0-rc.1`.

Every Vite+ prerelease uses the identifier `vp` and a counter `k` instead:

- On a stable Vite `X.Y.Z`, the base is the next stable core version, which
  is the version that the next stable release on that Vite would get. The core
  version is that base with `-vp.<k>` appended. `k` counts the published core
  prereleases with the same base, from 0. Vite+ `1.7.0-rc.0` and `1.7.0-rc.1`
  on Vite `8.4.0` publish `8.4.0-vp.0` and `8.4.0-vp.1`. A later `1.8.0-rc.0`
  on the same Vite, with no stable release between them, publishes
  `8.4.0-vp.2`. The next stable release publishes `8.4.0`.
- On a Vite prerelease `X.Y.Z-<pre>`, the core version is the Vite version
  with `.vp.<k>` appended. `k` counts the published core prereleases for that
  Vite prerelease. Vite `9.0.0-beta.3` gives `9.0.0-beta.3.vp.0`, then
  `9.0.0-beta.3.vp.1`.

The `vite-plus` prerelease identifiers do not appear in the core version. The
exact alias in the published CLI manifest links the two versions, as it does
for stable releases (see Relation to the `vite-plus` version).

The identifier `vp` sorts after `alpha`, `beta`, and `rc`, so every core
version is unique and in publish order. `semver@7.8.5` gives this order:

```text
9.0.0-beta.3 < 9.0.0-beta.3.vp.0 < 9.0.0-beta.3.vp.1 < 9.0.0-beta.4
  < 9.0.0-rc.1.vp.0 < 9.0.0-vp.0 < 9.0.0-vp.1 < 9.0.0
  < 9.0.1000-vp.0 < 9.0.1000
```

With `semver@7.8.5` and npm's strict prerelease rules, `9.0.0-beta.3.vp.0`
satisfies `^9.0.0-0` and `^9.0.0-beta.1`. This form was not tested in pnpm,
Yarn, or Bun.

A core prerelease on a stable Vite still fails strict peer checks in npm:
`8.4.0-vp.0` does not satisfy `^8.0.0`. Prereleases are opt-in, so this RFC
accepts the warnings for that period. Migration keeps the pnpm peer rule for
prerelease core versions (see Migration).

A Vite+ stable release bundles a stable Vite. Only a Vite+ prerelease may
bundle a Vite prerelease. The release check rejects a core prerelease on a
`vite-plus` version without a prerelease identifier, and a stable core on a
`vite-plus` prerelease. Between releases, main can bundle a Vite prerelease
with a stable `vite-plus` version (for example core `9.0.0-beta.3.vp.0` with
`vite-plus` `1.9.0`), so this pairing check runs only for a release.

Since `v1.0.0`, Vite+ published no prereleases. The last ones are
`v1.0.0-rc.0` and `v1.0.0-rc.1`. The prerelease rule matters mostly for the
Vite 9 beta period.

A new Vite major gives a new core major (`9.0.0` for Vite `9.0.0`). The
`vite-plus` version does not follow. A Vite major upgrade is a breaking change
for user plugins, so a `vite-plus` major release is the expected choice, but
that decision stays with the release policy.

### Relation to the `vite-plus` version

`vite-plus` stays on its own version line. The two versions are linked by the
exact alias in the published CLI manifest. pnpm already rewrites
`vite: workspace:@voidzero-dev/vite-plus-core@*` in
`packages/cli/package.json` to `npm:@voidzero-dev/vite-plus-core@<core version>`
at pack time, using the core package's own version. That mechanism needs no
change.

Code that assumes the two versions are equal must change:

| Location                                                | Current behavior                                                      | Required change                                                                                                                                   |
| ------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/cli/src/utils/constants.ts`                   | `VITE_PLUS_OVERRIDE_PACKAGES.vite` uses `VITE_PLUS_VERSION`           | Add `VITE_PLUS_CORE_VERSION = corePkg.version`, read from `packages/core/package.json` at build time, and use it for the alias                    |
| `packages/cli/src/resolve-core.ts`                      | Expects the core `version` to equal the installed `vite-plus` version | Parse the alias in the installed `vite-plus/package.json` `dependencies.vite`; skip the version check for a `workspace:` spec (repository layout) |
| `packages/cli/src/sync-versions/protocol.ts`, `plan.ts` | Maps `@voidzero-dev/vite-plus-core` to `toolchain.vitePlus`           | Add `vitePlusCore` from the `vite-plus-core` manifest node and map the core name to it                                                            |
| `packages/cli/src/migration/migrator/*`                 | Alias written through `VITE_PLUS_OVERRIDE_PACKAGES`                   | No change beyond the constant; the summary in `collectToolchainVersionChanges` already shows the raw Vite version                                 |
| `ecosystem-ci/patch-project.ts`                         | `VP_OVERRIDE_PACKAGES.vite` uses the `vite-plus` tarball version      | Use the packed core version                                                                                                                       |
| `packages/tools/src/install-global-cli.ts`              | Maps the alias version to `voidzero-dev-vite-plus-core-<version>.tgz` | No change; the alias already carries the core version after pack                                                                                  |
| `crates/vp_global_cli/src/commands/version.rs`          | Reads `bundledVersions` from the installed core manifest              | No change                                                                                                                                         |
| `packages/cli/toolchain.config.json`                    | `vite-plus-core` node uses `core-package` as its version source       | No change; `vp toolchain` shows the core version and the bundled Vite version as separate nodes                                                   |

The `vp upgrade` command in `crates/vp_global_cli/src/commands/upgrade/`
installs the global CLI from the published `vite-plus` package and its
lockfile. It does not read the core version.

### Where the version is computed and checked

Add a small module, `packages/tools/src/core-version.ts`, with pure
functions and a CLI entry:

- `encodeCoreVersion(vite, n)` returns the stable core version.
- `initialCoreVersion(vite)` returns the first core version for a bundled
  Vite: `n = 0` on a stable Vite, `<vite>.vp.0` on a Vite prerelease.
- `decodeCoreVersion(core)` returns `{ vite, n, k }` or an error.
- `nextCoreVersion({ vite, vitePlus, published })` returns the core version
  for a release. Its inputs are the bundled Vite version, the `vite-plus`
  version, and the published core versions. It applies the counter rules
  below.
- `assertCoreVersionMatchesVite(pkg, vitePlus, { release })` asserts that
  `pkg.version` decodes to `pkg.bundledVersions.vite`. With `release`, it also
  asserts the prerelease pairing, rejects `n = 999`, and requires
  `pkg.vitePlusVersion` to equal the released `vite-plus` version.
- `assertCoreVersionUnclaimed(published, core, vitePlus)` rejects a core
  version that the registry already holds for a different `vite-plus`
  version. A retry of the same release passes.

The name `checkCoreVersion` is not used, because
`packages/cli/src/resolve-core.ts` already defines a function with that name
for the installed-core check.

Counter rules in `nextCoreVersion`:

- Stable release: `n` is the highest `n` among published stable core versions
  with the same Vite version, plus 1, or 0 when none exists. If `n` would be
  higher than 998, the function fails.
- Prerelease on a stable Vite: the base is the version that the stable rule
  gives. `k` is the highest `k` among published `<base>-vp.*` versions, plus
  1, or 0 when none exists.
- Prerelease on a Vite prerelease: `k` is the highest `k` among published
  `<vite>.vp.*` versions, plus 1, or 0 when none exists.

The committed `packages/core/package.json` must always encode the committed
`bundledVersions.vite`. Two writers keep it consistent:

1. `.github/scripts/upgrade-deps.ts` already reads
   `packages/core/package.json` to sync `postcss` and `fsevents`. After a
   bundled Vite change it sets `version` to `initialCoreVersion(vite)`.
   Renovate ignores `vite` (`.github/renovate.json`), so this script is the
   only automated path that changes the bundled Vite.
2. `prepare_release.yml` runs `core-version.ts prepare`. It reads the
   published core versions
   (`npm view @voidzero-dev/vite-plus-core versions --json`), calls
   `nextCoreVersion` with the input `vite-plus` version, and writes the
   result and `vitePlusVersion` to the core manifest. The workflow prints
   both versions in the release PR body.

The counter comes from the registry when the release PR is prepared, so it
can be stale at publish time. Two release PRs prepared before either
publishes get the same core version, and a manual `vite-plus` bump skips the
selection. The publish step skips a version that is already published, so
the CLI could then pin a core tarball from another commit. The
`vitePlusVersion` field closes this gap: `release.yml` runs
`core-version.ts check --release`, which reads the published manifest of the
core version and fails unless the version is unpublished or was published for
the same `vite-plus` version. The field also maps a published core back to
its `vite-plus` release.

A unit test in `packages/core/__tests__/` runs `assertCoreVersionMatchesVite`
against the committed manifest. It skips the check when the version starts
with `0.0.0-commit.`, which the preview flow stamps. `packages/core/build.ts`
does not rewrite `version`: it runs after the release stamp and would race
with it.

### Local and CI builds

`test-vp-create.yml` and `reusable-ecosystem-ci.yml` set both packages to
`0.0.0` today (`npm pkg set version=0.0.0`), so that a correct install of the
local build is easy to detect. Core `0.0.0` does not encode a Vite version,
and peer checks fail against it. These flows would not test the path without a
`vite` peer rule that users get.

These flows will set core to `encodeCoreVersion(vite, 999)`, for example
`8.3.5999` for Vite `8.3.5`. They keep `0.0.0` for the CLI, which
`ecosystem-ci/verify-install.ts` and `test-vp-create.yml` assert. They set the
core version before the build, so the CLI build, its toolchain manifest, and
the packed `vite` alias all read it from `packages/core/package.json`. The
native build cache key does not include that file. `vp create` and
`vp migrate` in these flows write the `8.3.5999` alias and remove the `vite`
peer rule. No environment variable overrides the core version: the core
manifest is the only source.

`nextCoreVersion` never returns `n = 999`, and `release.yml` rejects it.
Thus a local version never reaches the registry, and the local registry
cannot mix it up with a published version.

### Release pipeline changes

| File                                                                                                                             | Change                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `.github/workflows/prepare_release.yml`                                                                                          | Compute the core version as described above; stop passing `VERSION` to `update_json packages/core/package.json`                                                                                                                                                                                                                                                                |
| `.github/workflows/release.yml`                                                                                                  | Read `CORE_VERSION` from `packages/core/package.json` at job start, and reject a version with `n = 999`; use it in "Publish core dependency tier", "Wait for core to propagate", the release notes, and the Discord embed. After publish, check that the core `latest` dist-tag equals `CORE_VERSION`. The version check against `unpkg.com/vite-plus@latest` stays on the CLI |
| `.github/workflows/reusable-release-build.yml`                                                                                   | Stamp `packages/core/package.json` only for preview versions (`0.0.0-commit.*`). A release checkout already carries the encoded version. Update the verify step to the same rule                                                                                                                                                                                               |
| `packages/core/build.ts`                                                                                                         | Pass `cliPkgJson.version` instead of `pkgJson.version` to `rewriteRolldownBindingRequires`. The platform packages publish at the CLI version, and the guard must compare against that version (`packages/core/build-support/rewrite-rolldown-binding.ts`)                                                                                                                      |
| `packages/cli/publish-native-addons.ts`                                                                                          | No change: it mirrors the CLI's platform pins into core by name and does not use the core version                                                                                                                                                                                                                                                                              |
| `.github/workflows/publish-preview.yml`, `publish-preview-register.yml`                                                          | No change: the bridge serves every workspace package as `0.0.0-commit.<sha>`, and the stamp before `pnpm pack` keeps the alias on that version. The sticky comment keeps printing one version for both packages                                                                                                                                                                |
| `.github/workflows/test-vp-create.yml`, `reusable-ecosystem-ci.yml`                                                              | Keep `0.0.0` for the CLI. Set core to `encodeCoreVersion(vite, 999)` before the build (see Local and CI builds)                                                                                                                                                                                                                                                                |
| `packages/tools/src/local-npm-registry.ts`, `pack-local-vite-plus.ts`                                                            | No change: both serve the packed tarball's own version                                                                                                                                                                                                                                                                                                                         |
| `.github/scripts/upgrade-deps.ts`                                                                                                | Reset the core version after a bundled Vite change                                                                                                                                                                                                                                                                                                                             |
| `.github/renovate.json`                                                                                                          | No change                                                                                                                                                                                                                                                                                                                                                                      |
| `.github/scripts/update-homebrew-formula.mjs`                                                                                    | Read the core version from the `vite` alias of the published `vite-plus` manifest, and wait for core at that version                                                                                                                                                                                                                                                           |
| `.claude/skills/release-manager/SKILL.md`                                                                                        | Read both versions from the release branch; publish checks use the core version for core                                                                                                                                                                                                                                                                                       |
| `packages/core/BUNDLING.md`                                                                                                      | Document the version scheme next to `bundledVersions`                                                                                                                                                                                                                                                                                                                          |
| `MAINTENANCE.md`                                                                                                                 | Mention the two versions in the release description                                                                                                                                                                                                                                                                                                                            |
| `README.md`, `packages/cli/README.md`, `docs/guide/local-cli.md`, `docs/guide/upgrade-project.md`, `docs/guide/migrate-rules.md` | Replace `<vite-plus-version>` in alias examples with `<core-version>` and point to `vp toolchain vite-plus-core`; describe the new pnpm peer rule behavior                                                                                                                                                                                                                     |

### Package-manager behavior

The experiment ran on macOS with pnpm `12.11.2`, npm `10.9.9`, Yarn
`4.18.1` (`nodeLinker: node-modules`), and Bun `1.4.4`. Each project
declared `vite` as an npm alias to a published package and local `file:`
consumers with one `vite` peer range each. `commander@8.3.0`,
`commander@8.0.0`, and `commander@9.0.0-0` stood in for future core versions;
`@voidzero-dev/vite-plus-core@1.4.0` is the current core;
`vite@8.3.0-beta.1` stood in for a prerelease core.

pnpm, npm, and Yarn resolve the alias and compare the target's manifest
version with the consumer's range. Their results for release versions agree:

| Alias target | `^8.0.0` | `^8.3.0` | `>=8.3.2` | `~8.3.0` | `^7.0.0` | `^8.0.0-0` | `>=8.0.0-0` |
| ------------ | -------- | -------- | --------- | -------- | -------- | ---------- | ----------- |
| `8.3.0`      | ok       | ok       | unmet     | ok       | unmet    | ok         | ok          |
| `8.0.0`      | ok       | unmet    | unmet     | unmet    | unmet    | ok         | ok          |
| core `1.4.0` | unmet    | unmet    | unmet     | unmet    | unmet    | unmet      | unmet       |

Prerelease targets differ by package manager:

| Alias target   | Manager | `^8.0.0` | `^8.3.0` | `^8.0.0-0` | `^8.3.0-0` | `>=8.0.0-0` | `>=8.0.0` |
| -------------- | ------- | -------- | -------- | ---------- | ---------- | ----------- | --------- |
| `8.3.0-beta.1` | npm     | unmet    | unmet    | unmet      | ok         | unmet       | unmet     |
| `8.3.0-beta.1` | pnpm    | ok       | unmet    | ok         | ok         | ok          | ok        |
| `8.3.0-beta.1` | Yarn    | ok       | ok       | ok         | ok         | ok          | ok        |
| `9.0.0-0`      | npm     | unmet    | unmet    | unmet      | unmet      | unmet       | unmet     |
| `9.0.0-0`      | pnpm    | unmet    | unmet    | unmet      | unmet      | ok          | ok        |
| `9.0.0-0`      | Yarn    | unmet    | unmet    | unmet      | unmet      | ok          | ok        |

npm follows strict semver prerelease rules. pnpm includes prereleases when
comparing. Yarn accepted `8.3.0-beta.1` for `^8.3.0`; the RFC records that
result without a cause.

Severity: pnpm prints `WARN Issues with peer dependencies found` and exits 0
unless `strict-peer-dependencies` is set; `pnpm peers check` lists each
consumer. npm prints `ERESOLVE overriding peer dependency` warnings and exited
0 in every case here. Yarn prints `YN0060` and exits 0. Bun printed no peer
diagnostic in any case, including a published consumer
(`@vitejs/plugin-react@6.1.2`, peer `^8.0.0`) against core `1.4.0`. Bun
resolved the alias and listed it as `vite@1.4.0`. Whether Bun checks peer
ranges in other layouts is unverified.

The current `vite-plus` manifest triggers the npm warning on its own, because
`vitest` is a dependency with a `vite` peer range. With core `8.3.5000` that
range is satisfied, so a project that depends only on `vite-plus` installs
without the warning.

## Migration

### Existing projects

`vp migrate` on a project that already uses Vite+ runs the toolchain upgrade
path (`docs/guide/upgrade-project.md`). The alias rewrite is unchanged in
shape: every `npm:@voidzero-dev/vite-plus-core@1.x` spec in dependencies,
`overrides`, `resolutions`, pnpm and Bun catalogs, and `pnpm-workspace.yaml`
becomes `npm:@voidzero-dev/vite-plus-core@<core version>`. The existing
`isProtocolPinnedSpec` rules keep `file:`, `link:`, and `workspace:` pins.

Peer rules change only for pnpm:

- `ensurePnpmPeerDependencyRules` in `vite-plus-bootstrap.ts`,
  `rewritePnpmWorkspaceYaml` in `catalog.ts`, and the writers in
  `orchestrators.ts` and `catalog.ts` stop adding `vite` to `allowAny` and
  `allowedVersions` when the target core version is a stable encoded version
  (major 8 or higher, no prerelease identifier). They remove `vite` from
  `allowAny` and remove `allowedVersions.vite` when its value is `'*'`. They
  delete an empty `peerDependencyRules` object or YAML map. Other entries
  stay, including `vitest`.
- The writers cannot tell whether an earlier migration or the user wrote
  `allowAny: [vite]` or `allowedVersions.vite: '*'`, so they remove both. On
  pnpm 10.2.0 and later, the only purpose of those entries is to accept the
  `1.x` alias. Migration only ever wrote `'*'`, so another
  `allowedVersions.vite` range is the user's own constraint and stays. `vp migrate` prints one line that names the removed
  rule, and `docs/guide/migrate-rules.md` describes the removal.
- When the target core version is not a stable encoded version (a preview
  `0.0.0-commit.<sha>` or a Vite+ prerelease such as `8.4.0-vp.0`), the
  writers keep the current behavior and write the `vite` rule.
- On pnpm before 10.2.0, the writers also keep the `vite` rule. These versions
  apply the `vite@*` alias override to peer ranges: every `vite` peer reads
  `vite@npm:@voidzero-dev/vite-plus-core@<version>` and fails against the
  installed core, also for `vitest` and `@vitest/mocker`. pnpm 10.2.0 and
  later check peers against the alias target version.
- `pnpmPeerDependencyRulesSatisfyVitePlus` reports a project as pending when
  a `vite` entry exists and the target core is a stable encoded version, or
  when the entry is missing and the target core is not.

npm, Yarn, and Bun have no peer suppression to remove. On npm, the
`overrides.vite` entry also rewrites peer edges, so a migrated npm project does
not report an incompatible `vite` peer: the third clause of the goal ("an
incompatible plugin still fails") holds on pnpm and Yarn, not on npm. Bun
reports no peer problems in any layout. The override stays on every package
manager, because it keeps one core instance for direct, transitive, and peer
`vite` dependencies. Their alias handling is unchanged: npm keeps the direct dependency and `overrides.vite` on the same
spec (`EOVERRIDE` rule), `npm-reinstall.ts` keeps moving stale `vite`
installs aside by package name, Yarn keeps `resolutions.vite`, and Bun keeps
the mirrored direct `vite` dependency.

The final install step refreshes lockfiles. Every package manager sees the
alias target version change from `1.x` to `8.x` and re-resolves it; no
lockfile edit is needed beyond the existing npm stale-install handling.

After migration, a plugin with an incompatible range (`vite: ^7.0.0`) produces
a peer warning for the first time. This is the intended result. The migrate
rules document must say so.

### Downgrade

A project on core `8.3.5000` can move back to `vite-plus@1.4.0` by running
that CLI's `vp migrate`. The older CLI rewrites the alias to
`npm:@voidzero-dev/vite-plus-core@1.4.0` and re-adds the pnpm `vite` peer
rule, because its logic is self-contained. Its `resolveCore` check compares
the core version with its own version, so it reports a mismatch until the
alias is realigned. The newer CLI reports the same for a project still on
`1.4.0`: "Expected `@voidzero-dev/vite-plus-core@8.3.5000`, found `1.4.0`.
Run `vp migrate`." Exact pins make the registry ordering irrelevant for the
downgrade.

### Fixtures and documentation

Snapshot fixtures under `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/`
that record `peerDependencyRules.allowAny: [vite, vitest]` or the alias
version need re-recording with `UPDATE_SNAPSHOTS=1`. The redaction in
`crates/vp_cli_snapshots/tests/cli_snapshots/redact.rs` already masks
`@voidzero-dev/vite-plus-core@<version>` with a prerelease-aware pattern, so
local-flavor snapshots do not depend on the chosen encoding.

## Alternatives

### Core major = Vite major, minor and patch from `vite-plus`

Simplest for the pipeline: one `sed` with a different first component. It
fails minor ranges in both directions, as shown in the comparison table. The
release dates in this repository already show the drift.

### Plain release counter in the patch

Correct for minor ranges and easier to read (`8.3.7`). It misreports patch
floors: `>=8.3.2` fails against `8.3.0` although the bundled Vite is `8.3.5`.
The encoded form keeps patch floors correct up to the bundled patch.

The encoded form does not make every core version look different from a Vite
version. On Vite patch 0, the core patch is `n` alone (`8.4.0`, `8.4.1`,
...), and core `8.4.1` looks like Vite `8.4.1`. Decoding still gives the
correct Vite version (`floor(1 / 1000) = 0`). Peer checks stay in the residual
error class of the selected scheme: `>=8.4.1` passes against core `8.4.1`.

### Exact Vite version with a prerelease tag or build metadata

Fails npm peer checks and cannot publish two Vite+ releases on one Vite
patch. See the comparison.

### Encode the counter in the Vite prerelease number

A core on a Vite prerelease could use the same `* 1000` rule as a stable
core: Vite `9.0.0-beta.3` would give `9.0.0-beta.3000`, then
`9.0.0-beta.3001`. This gives one shape for every core version and one
identifier less.

The stable form needs the counter in the patch, because an extra identifier
would make the version a prerelease, and npm does not match a prerelease
against ranges such as `^8.0.0`. A Vite prerelease is already a prerelease,
so the counter can follow it as `.vp.<k>`. That version sorts directly after
the Vite version and before the next one. The encoded number sorts after
later Vite prereleases:

```text
9.0.0-beta.3 < 9.0.0-beta.3.vp.0 < 9.0.0-beta.4
9.0.0-beta.3 < 9.0.0-beta.4 < 9.0.0-beta.3000
```

`semver@7.8.6` gives these peer results:

| Consumer range   | `9.0.0-beta.3.vp.0` | `9.0.0-beta.3000` |
| ---------------- | ------------------- | ----------------- |
| `^9.0.0-beta.1`  | pass                | pass              |
| `^9.0.0-beta.3`  | pass                | pass              |
| `^9.0.0-beta.5`  | fail                | pass (wrong)      |
| `>=9.0.0-beta.5` | fail                | pass (wrong)      |
| `^9.0.0-rc.0`    | fail                | fail              |

The `.vp.<k>` form has no residual false result for prerelease floors. The
encoded form adds the stable scheme's residual error to them. Plugins use such
floors during a Vite beta: `@sveltejs/vite-plugin-svelte` declares
`^8.0.0-beta.7 || ^8.0.0`. A core that bundles Vite `8.0.0-beta.3` as
`8.0.0-beta.3000` would pass that check although its Vite is too old.
`oxlint-tsgolint` publishes no prerelease versions, so its convention does
not decide this case.

### Publish core only when the bundled Vite changes

Core would use the exact Vite version. Core also bundles Rolldown, tsdown,
and the Vite+ build of the Vite source, so a Vite+ release that changes any of
these without a Vite change could not publish a new core.

### Keep the version and ask consumers to widen peer ranges

Plugins cannot know about a `1.x` alias target. The npm warning comes from
`vite-plus`'s own dependency tree, so no consumer change removes it.

### Put the bundled Vite version in `peerDependenciesMeta` or a custom field

Package managers do not read a custom field for peer resolution. Only the
manifest `version` takes part in the check.

## Risks

- Confusion with upstream Vite. `npm ls vite` prints
  `vite@npm:@voidzero-dev/vite-plus-core@8.3.5000`, and readers may take
  `8.3.5000` for a Vite version. On Vite patch 0, the core version (`8.4.1`)
  looks exactly like a Vite version. The decoding rule is one division, and
  `vp toolchain` prints the real Vite version. Documentation must show both.
- Bundled Vite downgrade. If Vite+ goes back to an older Vite version, the
  core version goes down, for example from `8.4.0` to `8.3.5001`. The counter
  rules still give a unique version, and exact pins make the lower version
  install correctly. `release.yml` publishes core with `--tag latest`. How
  npm sets `latest` when the published version is lower than the current
  `latest` is unverified. The dist-tag check after publish (see Release
  pipeline changes) makes a wrong tag visible.
- Registry ordering. `8.3.5000` sorts above every published `1.x` version, so
  `latest` moves forward as usual and a range on core would jump majors.
  Vite+ documents exact pins only. The stale `alpha` and `test` dist-tags
  point at `0.x` versions and can be removed or left alone.
- Peer warnings appear for real incompatibilities after migration. Projects
  that relied on the broad rule to hide a `^7.0.0` plugin see a warning or,
  with `strict-peer-dependencies`, an install failure. The migrate output
  should name the change. For one plugin, the migrate rules document
  recommends the pnpm override `'vite-plugin-foo>vite:peer': '*'` on pnpm
  12.17.0 and later, and a scoped `allowedVersions` entry on earlier pnpm.
- pnpm 12.17.0 deprecates `peerDependencyRules` (pnpm/pnpm#17457). The setting
  still works for backward compatibility. Migration still writes it in three
  cases: the `vite` rule for preview and prerelease cores and for pnpm before
  10.2.0, and the `vitest` rule. The `:peer` override is not a drop-in
  replacement, because it rewrites the declared peer ranges instead of only
  hiding the report, and pnpm before 12.17.0 does not support it. The `vitest`
  rule is redundant (the `vitest` override already sets the version that peer
  checks see), so the follow-up that removes it also removes one of the three
  cases. The data in #2806 confirms that a workspace on `vite-plus` 1.5.0 with
  Vitest 5.1.0 needs no `vitest` rule.
- Vite+ prereleases still warn in npm: `8.4.0-vp.0` does not satisfy
  `^8.0.0`. Accepted; see Prereleases.
- Preview builds still warn in npm. The bridge forces `0.0.0-commit.<sha>`,
  and the release-manager skill already lists this class of artifact as a
  non-blocker. Migration keeps the pnpm rule for preview cores. Thus preview
  builds cannot show the new peer behavior, and the preview check in
  `.claude/skills/test-pkg-pr-new-migrate/` does not test it. Verification
  before a release uses the local registry with the `n = 999` local version
  (see Local and CI builds).
- Tools that read the `vite` version. Vite's `version` export is rewritten in
  `packages/core/build.ts` to import `../../package.json` relative to Vite's
  `src/node/constants.ts`, which is Vite's own manifest, so the export keeps
  the real Vite version. This was checked by reading the build script; the
  built output was not inspected in this worktree. Code that reads
  `node_modules/vite/package.json` directly, such as
  `readInstalledRawViteVersion` in `vite-plus-bootstrap.ts`, keys on
  `name === '@voidzero-dev/vite-plus-core'` and reads `bundledVersions.vite`.
  Third-party code that reads `version` from that file now sees `8.3.5000`
  instead of `1.4.0`, which is closer to the truth; whether any such code
  gates features on it is unverified.
- Binding guards. The Rolldown binding guard in core compares the installed
  platform package version with a baked version. Today it bakes the core
  version, which equals the CLI version. The build must bake the CLI version
  after this change, or every release build fails the guard.
- Counter lookup needs the registry. `prepare_release.yml` already needs the
  network for `cargo check`. A registry failure must stop the job with a
  clear error rather than guess `n = 0`.
- Pipelines that pass one `VERSION` for every package. The list in Release
  pipeline changes covers the files found by searching for
  `vite-plus-core`; a missed `${VERSION}` reference would publish or wait for
  a core version that does not exist. `wait-for-npm-packages.ts` then fails
  the job, so the error is visible.

## Other aliased packages

Only `vite` is aliased in user projects. `vitest` is pinned to its real
upstream version, so peer checks against it are accurate. Rolldown and tsdown
are bundled into core and exposed as `vite/rolldown` and `vite/pack`; they are
not aliased, and consumers do not declare peers on them through Vite+. The
removed `@voidzero-dev/vite-plus-test` alias no longer exists. No other
package has the same problem, and none is in scope.

## Decisions from review

- Local and CI builds use the reserved `n = 999`, and the release counter stops
  at 998 (see Local and CI builds).
- Vite+ prereleases use the `vp.<k>` identifier instead of a copy of the
  `vite-plus` prerelease identifiers (see Prereleases and new Vite majors).
- The `vitest` peer rule is removed in a follow-up with its own fixture
  updates. `catalog.ts` already removes it when the project does not use
  Vitest. The follow-up covers projects that use Vitest.
- The counter comes from a registry lookup in `prepare_release.yml`, not from
  a committed counter. The committed manifest only has to encode the bundled
  Vite version.

## Open questions

1. Whether a Vite major upgrade is a `vite-plus` major release.
   Recommendation: yes, decided in the release policy, not here.

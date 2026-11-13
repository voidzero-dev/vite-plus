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
bundles:

```text
stable Vite+ release:          <vite major>.<vite minor>.<vite patch * 1000 + n>
Vite+ prerelease, stable Vite: <next stable core version>-vp.<k>
Vite+ prerelease, Vite pre:    <vite version>.vp.<k>
```

`n` counts the stable Vite+ releases on one Vite version, and `k` counts the
Vite+ prereleases on one base. Both go from 0 to 998. The value 999 is
reserved for local and CI builds. The bundled Vite patch is
`floor(core patch / 1000)`. This is the convention of `oxlint-tsgolint`, whose
first release on TypeScript `7.0.2` is `7.0.2000`
([tsgolint versioning](https://github.com/oxc-project/tsgolint#versioning)).

| Vite+ release     | Bundled Vite   | Core version          |
| ----------------- | -------------- | --------------------- |
| `1.7.0` (first)   | `8.3.8`        | `8.3.8000`            |
| `1.7.1`           | `8.3.8`        | `8.3.8001`            |
| `1.8.0`           | `8.3.9`        | `8.3.9000`            |
| `1.9.0-rc.0`      | `8.4.0`        | `8.4.0-vp.0`          |
| `1.9.0`           | `8.4.0`        | `8.4.0`               |
| `1.9.1`           | `8.4.1`        | `8.4.1000`            |
| `2.0.0-beta.0`    | `9.0.0-beta.3` | `9.0.0-beta.3.vp.0`   |
| Local or CI build | `8.3.8`        | `8.3.8999`            |
| Local or CI build | `9.0.0-beta.3` | `9.0.0-beta.3.vp.999` |

`vite-plus` keeps its own version line (`1.x`). Projects keep the exact alias
`vite: npm:@voidzero-dev/vite-plus-core@<core version>`, and `vp migrate`
removes the broad pnpm `vite` peer rule. For an upgrade from `1.6.0`:

```diff
 overrides:
   vite@*: 'catalog:'
-peerDependencyRules:
-  allowAny:
-    - vite
-  allowedVersions:
-    vite: '*'
 catalog:
-  vite: npm:@voidzero-dev/vite-plus-core@1.6.0
+  vite: npm:@voidzero-dev/vite-plus-core@8.3.8000
-  vite-plus: 1.6.0
+  vite-plus: 1.7.0
```

## Problem

Package managers compare the version of the alias target with each `vite`
peer range. Core `1.6.0` fails every current range: `^8.0.0`
(`@vitejs/plugin-react`), `^6.4.0 || ^7.0.0 || ^8.0.0` (Vitest), and
`>=8.0.0` (`@tanstack/react-start`).

- pnpm projects need `peerDependencyRules` with `allowAny: [vite]` and
  `allowedVersions.vite: '*'`, which `vp migrate` writes. These rules also hide
  real incompatibilities, such as a plugin that declares `vite: ^7.0.0`.
- npm has no per-package suppression. The `vite-plus` package alone depends on
  the core alias and on Vitest, whose peer range rejects `1.x`, so every
  `npm install` prints `ERESOLVE overriding peer dependency`.
- The release pipeline gives every package the same `VERSION`.

## Goals and scope

- A package manager reports a `vite` peer as satisfied when the bundled Vite
  satisfies the range, and still reports a range that excludes the bundled
  Vite major or minor. Patch floors resolve correctly up to the bundled patch.
- Migration stops writing the broad `vite` peer rule and removes the one that
  it wrote earlier. Rules for other packages stay.
- The core version and the bundled Vite version map to each other without a
  lookup table, and CI checks that the committed core version encodes
  `bundledVersions.vite`.

Out of scope: the `vitest` peer rule (`vitest` is pinned to its real version,
so a follow-up removes the rule), the `vite-plus` version policy, and the
preview registry bridge, which serves every package as `0.0.0-commit.<sha>`.

## Design

### Version scheme

The scheme must resolve `^8.0.0`, `^8.3.0`, `~8.3.0`, and `>=8.3.2` like the
bundled Vite. Results for bundled Vite `8.3.5`:

| Consumer range | Bundled `8.3.5` | A `8.4.0`    | B `8.3.0`    | B+ `8.3.5000` | C `8.3.5-vp.1.4.0` (npm) |
| -------------- | --------------- | ------------ | ------------ | ------------- | ------------------------ |
| `^8.0.0`       | pass            | pass         | pass         | pass          | fail (wrong)             |
| `^8.3.0`       | pass            | pass         | pass         | pass          | fail (wrong)             |
| `>=8.3.2`      | pass            | pass         | fail (wrong) | pass          | fail (wrong)             |
| `~8.3.0`       | pass            | fail (wrong) | pass         | pass          | fail (wrong)             |
| `^7.0.0`       | fail            | fail         | fail         | fail          | fail                     |
| `^8.4.0`       | fail            | pass (wrong) | fail         | fail          | fail                     |
| `>=8.3.6`      | fail            | pass (wrong) | fail         | pass (wrong)  | fail                     |

- **A** (core major = Vite major, minor and patch from `vite-plus`) fails
  minor ranges, because the `vite-plus` minor and the Vite minor drift apart.
- **B** (Vite major and minor, plain counter in the patch) fails patch floors,
  because the counter is not related to the Vite patch.
- **C** (exact Vite version with a prerelease tag) fails in npm, because npm
  matches a prerelease only against comparators with the same
  `major.minor.patch`. Build metadata (`8.3.5+a`) cannot publish two Vite+
  releases on one Vite patch, and `v1.2.0` and `v1.3.0` both bundled `8.3.4`.
- **B+** (selected) keeps three rare false results: a patch floor above the
  bundled patch (`>=8.3.6`), a patch ceiling (`<8.3.10` fails against
  `8.3.5000`), and an exact pin (`8.3.5`).

On Vite patch 0, the core patch is `n` alone, so core `8.4.1` looks like Vite
`8.4.1`. It still decodes to Vite `8.4.0`.

The core major and minor follow Vite, not `vite-plus`. Releases on one Vite
version change only the counter:

| `vite-plus`    | Bundled Vite | Core version    | Reason                                |
| -------------- | ------------ | --------------- | ------------------------------------- |
| `1.7.0`        | `8.3.8`      | `8.3.8000`      | First stable release on Vite `8.3.8`  |
| `1.8.0-beta.0` | `8.3.8`      | `8.3.8001-vp.0` | The base is the next stable core      |
| `1.8.0-beta.1` | `8.3.8`      | `8.3.8001-vp.1` | Same base, next `k`                   |
| `1.8.0`        | `8.3.8`      | `8.3.8001`      | The stable release takes the base     |
| `1.9.0`        | `8.3.8`      | `8.3.8002`      | Next counter                          |
| `2.0.0`        | `8.3.8`      | `8.3.8003`      | A `vite-plus` major is a counter step |

So the core version does not signal a `vite-plus` breaking change. The CLI and
`vp migrate` use exact versions only, so no project gets a new core through a
range.

### Prereleases

A Vite+ prerelease uses the identifier `vp` and the counter `k`. It does not
copy the `vite-plus` identifiers (`rc.0`): two prereleases such as
`1.9.0-rc.0` and `1.10.0-rc.0` on one Vite would get the same core version,
and on Vite `9.0.0` a copied `9.0.0-rc.0` would sort below Vite `9.0.0-rc.1`.

`vp` sorts after `alpha`, `beta`, and `rc`, so the order follows the publish
order (`semver@7.8.5`):

```text
9.0.0-beta.3 < 9.0.0-beta.3.vp.0 < 9.0.0-beta.3.vp.1 < 9.0.0-beta.4
  < 9.0.0-vp.0 < 9.0.0 < 9.0.1000-vp.0 < 9.0.1000
```

A Vite+ stable release bundles a stable Vite. The release check rejects a core
prerelease for a stable `vite-plus` version, and the reverse. Between
releases, main can bundle a Vite prerelease with a stable `vite-plus` version.

A core prerelease on a stable Vite (`8.4.0-vp.0`) does not satisfy `^8.0.0` in
npm. Prereleases are opt-in, so this RFC accepts the warnings, and migration
keeps the pnpm `vite` rule for prerelease cores. A new Vite major gives a new
core major. Whether it is also a `vite-plus` major is a release policy
decision.

### Release pipeline

`packages/tools/src/core-version.ts` holds the scheme as pure functions and
three commands:

- `prepare --vite-plus <version>` runs in `prepare_release.yml`. It fetches
  the core packument, selects the next counter, and writes the core version
  and `vitePlusVersion` to `packages/core/package.json`. A registry
  failure stops it. It does not select counter 0.
- `check --release` runs in `release.yml`, in the `check` job and again before
  publishing. It requires the version to encode `bundledVersions.vite`,
  `vitePlusVersion` to equal the CLI version, and the prerelease pairing to
  match. It also rejects counter 999.
- `local` prints the version for local and CI builds.

`.github/scripts/upgrade-deps.ts` resets the version to counter 0 when the
bundled Vite changes. A unit test in `packages/core` requires the committed
version to encode `bundledVersions.vite`, so the committed version is always
valid. It skips the `0.0.0-commit.*` versions of preview builds.

The counter comes from the registry when the release PR is prepared, so it can
be stale at publish time. These rules keep a published CLI from pinning a core
that belongs to another release:

- The counters count every version number that npm has used, including
  unpublished numbers in the packument `time` field. npm never accepts such a
  number again.
- `check --release` fails when npm already has the version for another
  `vite-plus` version, or when the number was unpublished. A retry of the same
  release passes. This catches two release PRs that selected the same counter,
  and a manual `vite-plus` bump.
- `release.yml` runs one release at a time (`concurrency: release`, no
  cancel), because npm can hide a new version for some minutes after the
  upload. GitHub keeps one pending run per group, so a third run replaces the
  pending one and needs a manual rerun.
- `packages/core/package.json` is a `release.yml` trigger. After a rejected
  check, a new Prepare Release run for the same `vite-plus` version opens a PR
  that changes only that file, and its merge starts the release again.

Other pipeline changes:

- `release.yml` publishes, waits for, and announces core at its own version,
  and checks the core `latest` dist-tag after publish.
- `reusable-release-build.yml` stamps the core version only for preview
  builds. The Rolldown binding guard in `packages/core/build.ts` bakes the CLI
  version, because the platform packages publish at the CLI version.
- `update-homebrew-formula.mjs` reads the core version from the `vite` alias
  of the published `vite-plus`.
- `test-vp-create.yml` and `reusable-ecosystem-ci.yml` set core to the
  `local` version (`8.3.8999`) and keep the CLI at `0.0.0`. Before, core was
  `0.0.0`, which fails every peer check. Other local builds, such as
  `pnpm bootstrap-cli` and the snapshot runner, pack the committed version, as
  before.

`packages/core/package.json` is the only source of the core version. No
environment variable overrides it.

### CLI

- `packages/cli/src/utils/constants.ts` reads `VITE_PLUS_CORE_VERSION` from
  the core manifest at build time and uses it for the alias.
- `packages/cli/src/resolve-core.ts` compares the installed core with the
  exact version in the CLI's own `vite` alias. In the repository layout
  (`workspace:` spec), it compares the project core with the core that the CLI
  resolves.
- `vp toolchain` shows the core version and the bundled Vite version on
  separate lines. `sync-versions` reads the core version from the
  `vite-plus-core` node.

pnpm already rewrites the `workspace:` alias in `packages/cli/package.json` to
the core version at pack time. `vp upgrade` and `vp --version` need no change.

### Package-manager behavior

An experiment with pnpm `12.11.2`, npm `10.9.9`, Yarn `4.18.1`, and Bun
`1.4.4` used published packages at the candidate versions as alias targets.
pnpm, npm, and Yarn agree for release versions:

| Alias target | `^8.0.0` | `^8.3.0` | `>=8.3.2` | `~8.3.0` | `^7.0.0` |
| ------------ | -------- | -------- | --------- | -------- | -------- |
| `8.3.0`      | ok       | ok       | unmet     | ok       | unmet    |
| `8.0.0`      | ok       | unmet    | unmet     | unmet    | unmet    |
| core `1.4.0` | unmet    | unmet    | unmet     | unmet    | unmet    |

For prerelease targets, npm follows strict semver rules, and pnpm and Yarn
are more lenient. pnpm prints a warning and exits 0 unless
`strict-peer-dependencies` is set. Yarn prints `YN0060`, and npm prints
`ERESOLVE` warnings. Both exit 0. Bun reports no peer problems.

## Migration

`vp migrate` on an existing Vite+ project rewrites every core alias (in
dependencies, overrides, resolutions, and catalogs) to the new core version,
as before. `file:`, `link:`, and `workspace:` pins stay.

For pnpm, the writers in `vite-plus-bootstrap.ts`, `catalog.ts`, and
`orchestrators.ts` remove `vite` from `allowAny` and remove
`allowedVersions.vite: '*'` when the target core is a stable encoded version.
They delete emptied maps, and `vp migrate` prints one line that names the
removed rule. Migration cannot tell whether it or the user wrote these
entries, so it removes both. Another `allowedVersions.vite` range is the
user's own rule and stays. The rule stays in two cases:

- A preview or prerelease core, which fails strict peer checks.
- pnpm before 10.2.0, which applies the `vite@*` alias override to peer ranges,
  so every `vite` peer fails against the core.

npm, Yarn, and Bun have no peer rule to remove. The `vite` overrides and
resolutions stay on every package manager, because they keep one core
instance. On npm, `overrides.vite` also rewrites peer edges, so npm does not
report an incompatible `vite` peer. The goal "an incompatible plugin still
fails" holds on pnpm and Yarn only.

After migration, a plugin with an incompatible range (`vite: ^7.0.0`) produces
a peer warning. This is the intended result. The migration rules document
recommends a rule for that plugin only: the pnpm override
`'vite-plugin-foo>vite:peer': '*'` on pnpm 12.17.0 and later, or a scoped
`allowedVersions` entry on earlier pnpm.

A downgrade with the older CLI's `vp migrate` rewrites the alias to `1.x` and
adds the rule again. Both CLIs report a core mismatch until the alias matches
their own core.

## Alternatives

- **Encode the counter in the Vite prerelease number**
  (`9.0.0-beta.3000`). It gives one shape for all versions, but
  `9.0.0-beta.3000` sorts after `9.0.0-beta.4` and passes
  `^9.0.0-beta.5` although its Vite is too old. Plugins use such floors during
  a Vite beta, for example `@sveltejs/vite-plugin-svelte` declares
  `^8.0.0-beta.7 || ^8.0.0`. `9.0.0-beta.3.vp.0` has no false result there.
- **Publish core only when the bundled Vite changes.** Core also bundles
  Rolldown, tsdown, and Vite+ patches, so a Vite+ release without a Vite
  change could not publish a new core.
- **Ask plugins to widen their peer ranges.** Plugins cannot know about a
  `1.x` alias target, and the npm warning comes from the `vite-plus`
  dependency tree itself.
- **Put the Vite version in a custom field.** Package managers check only the
  manifest `version`.

## Risks

- **Confusion with upstream Vite.** `npm ls vite` prints `8.3.8000`, which
  readers can take for a Vite version. `vp toolchain` prints the real Vite
  version.
- **Bundled Vite downgrade.** The core version goes down, for example from
  `8.4.0` to `8.3.8001`. How npm sets `latest` for a lower version is
  unverified. The dist-tag check after publish shows a wrong tag.
- **Ranges on the core package.** A `vite-plus` breaking change can be a core
  patch step, so a range such as `^8.3.8000` could get a breaking change. Vite+
  supports exact versions only.
- **Peer warnings for real incompatibilities.** Projects that relied on the
  broad rule see warnings, or install failures with strict peer checks.
- **Tools that read the Vite version.** Vite's `version` export keeps the real
  Vite version: importing the built core gives the bundled Vite version.
  Third-party code that reads `node_modules/vite/package.json` sees `8.3.8000`
  instead of `1.x`.
- **Snapshots follow the kind of bundled Vite.** When the bundled Vite becomes
  a prerelease (expected for Vite 9), the committed core becomes
  `<vite>.vp.0`, migration keeps the pnpm rule, and the snapshots that print
  pnpm settings change. The upgrade-deps PR must record them again.

## Open questions

1. Whether a Vite major upgrade is a `vite-plus` major release.
   Recommendation: yes, decided in the release policy, not here.

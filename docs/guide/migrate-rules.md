# Migration Rules

This guide explains how `vp migrate` updates dependencies, source imports, and
package-manager configuration in existing Vite+ projects. See the
[migration guide](./migrate.md) for the complete command overview.

## Before You Migrate

1. Run `vp upgrade` before migrating an existing Vite+ project. A stale local
   CLI does not contain the new migration rules; migration delegates to the
   global CLI when the local version is older.
2. Upgrade the project to Vite 8+ and Vitest 4.1+ when necessary.
3. Run `vp migrate` from the workspace root. Use `--no-interactive` in
   automated environments.
4. Review every changed manifest, package-manager config, source rewrite, and
   generated lockfile.
5. Validate with `vp install`, `vp check`, `vp test`, and `vp build`.

Running the migration again after a successful migration should not produce
another diff.

## Dependency Versions

- `vite-plus` is pinned to the concrete version of the CLI running the
  migration, not the `latest` dist-tag.
- The `vite` alias must target `@voidzero-dev/vite-plus-core` from the same
  Vite+ release.
- A catalog-backed manifest may contain `catalog:` or an existing named catalog
  reference. The referenced catalog value must still be updated to the concrete
  toolchain target.
- Preserve deliberate protocol pins such as `workspace:`, `file:`, `link:`,
  `npm:`, `github:`, Git URLs, and HTTP URLs.
- Reconcile every workspace package, not only the root manifest. Shared
  overrides and catalogs stay at the workspace root; direct peer providers
  belong in each package that needs them.

## Dependency Changes

| Dependency                     | Migration rule                                                                                                                                                                                                                                          |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vite-plus`                    | Add it where the package is migrated. Re-pin plain ranges to the current concrete target, directly or through a catalog. Preserve deliberate protocol pins.                                                                                             |
| `vite`                         | Keep existing declarations. With pnpm, add a direct dev dependency to every package that depends on `vite-plus` and does not already declare `vite`. Point managed edges and the shared override to the matching `@voidzero-dev/vite-plus-core` target. |
| `vitest`                       | Remove it in the common node-mode case because `vite-plus` provides it transitively. Keep or add an exact bundled version only in packages with direct Vitest requirements.                                                                             |
| `@vitest/*`                    | Align lockstep packages that the project directly lists to the bundled Vitest version. Prefer the package's existing catalog reference when its catalog owns that package; otherwise write the concrete version.                                        |
| `@voidzero-dev/vite-plus-test` | Remove all dependency, override, resolution, and catalog aliases. Rewrite imports to the current `vite-plus/test*` surface.                                                                                                                             |

### Vite and Overrides

Package-manager overrides do not synthesize dependency edges. Under pnpm, every
package that lists `vite-plus` in `dependencies` or `devDependencies` must also
declare `vite`, unless it already has a `vite` entry in `dependencies`,
`devDependencies`, `optionalDependencies`, or `peerDependencies`. Otherwise,
pnpm can auto-install upstream Vite to satisfy Vitest's required `vite` peer,
creating separate Vite+, Vite, and Vitest instances. `vp migrate` adds a missing
`vite` entry to `devDependencies`; the workspace override redirects it to Vite+
core.

Do not remove a direct `vite` declaration merely because a root override exists.
Normalize existing plain or stale aliases while retaining named catalog
references. The general rule above is specific to pnpm. Bun mirrors its core
alias as a direct dependency for its peer resolver, while npm browser-provider
layouts may need a top-level edge so nested Vitest packages can resolve `vite`.

### When Vitest Is Directly Required

Keep or add package-local `vitest` at the exact bundled version when any of the
following is true:

- an installed dependency has a non-optional `vitest` peer, whether exact or a
  range;
- the package uses Vitest browser mode or an opt-in browser provider;
- source or TypeScript configuration retains an upstream `vitest` reference;
- the package declares `@nuxt/test-utils`; or
- dependency metadata is unavailable and an existing direct `vitest` might be
  satisfying an unknown required peer.

`vp migrate` checks installed peer metadata, so integrations such as
`vite-plugin-gherkin` are handled even though their names do not contain
`vitest`.

When a package directly requires Vitest:

- add `vitest` to that package, not indiscriminately to every workspace package;
- use the existing catalog reference when supported, otherwise use the exact
  bundled version; and
- keep a matching workspace override or resolution so the graph uses one
  Vitest version.

A peer declaration alone does not install Vitest. If a surviving
`peerDependencies.vitest` uses a catalog entry that migration will remove,
resolve it to the public peer range first.

### Vitest Ecosystem Packages

Official current `@vitest/*` packages generally publish in lockstep with
Vitest. Align packages the project directly installs, including
`@vitest/coverage-v8`, `@vitest/coverage-istanbul`, `@vitest/ui`, and
`@vitest/web-worker`.

Catalog handling is package-specific:

- preserve `catalog:` and named `catalog:<name>` dependency references when the
  corresponding catalog already defines that package;
- update that catalog entry to the bundled Vitest version; and
- use the concrete bundled version when no catalog owns the package.

Do not align independently versioned or obsolete packages:

- `@vitest/eslint-plugin` has its own version line;
- `@vitest/coverage-c8` stopped at an older release and has no Vitest 4 version;
  and
- third-party `vitest-*` integrations keep their own compatible package
  versions, although their required Vitest peer may require direct provisioning.

The base `@vitest/browser` runtime and `@vitest/browser-preview` are bundled by
Vite+ and should be removed as direct dependencies. The Playwright and
WebdriverIO providers remain opt-in. Preserve an existing catalog reference when
its catalog owns the provider. When migration injects a provider into a
catalog-capable project, it uses the preferred catalog and adds the provider at
the bundled Vitest version. Otherwise, it writes the concrete bundled version.
Ensure the provider's `playwright` or `webdriverio` peer is also installed.

Migration detects providers before rewriting imports. This includes legacy
projects that aliased `vitest` to `@voidzero-dev/vite-plus-test` and import from
`vitest/browser-<provider>`, `vitest/browser/providers/<provider>`, or
`vitest/plugins/browser-<provider>`. These imports still cause the corresponding
`@vitest/browser-playwright` or `@vitest/browser-webdriverio` dependency and its
framework peer to be installed.

Object-valued nested npm and Bun overrides are preserved because they are
user-defined scopes rather than scalar version pins.

## Source Rewrite Rules

- Rewrite ordinary `vite` and `vite/*` imports to `vite-plus`, except in plugin
  packages. A package is treated as a plugin when its unscoped name starts with
  `vite-plugin-` (the Vite plugin naming convention) or `unplugin-` (the unplugin
  naming convention), or it declares `vite` in `peerDependencies` or
  `dependencies`. Preserving the upstream `vite` import keeps a published plugin
  usable by plain Vite projects; in a Vite+ project `vite` still resolves through
  the `@voidzero-dev/vite-plus-core` alias, so the import works in both
  ecosystems. The skip is scoped to `vite` only.
- Rewrite ordinary `vitest` and `vitest/*` imports to `vite-plus/test*`.
- Detect legacy Playwright and WebdriverIO provider imports before applying that
  rewrite so their optional provider dependencies are not lost.
- Rewrite scoped browser imports to the corresponding
  `vite-plus/test/browser*` exports and provision opt-in providers when needed.
- Leave existing `vite-plus/test*` imports unchanged.
- Do not rewrite `declare module 'vitest'` or
  `declare module '@vitest/browser*'`. Module augmentation must retain the
  upstream module identity.
- Retained references such as `compilerOptions.types`, `require.resolve`,
  `import.meta.resolve`, and `vitest/package.json` require package-local Vitest.
- In a package that declares `@nuxt/test-utils`, preserve every `vitest` and
  `vitest/*` module specifier package-wide. Its transform requires the upstream
  identity and can otherwise inject a duplicate `vi` import. This exception
  does not apply to sibling packages or scoped `@vitest/browser*` imports.

The `prefer-vite-plus-imports` lint rule follows the same Nuxt exception, so
lint autofix preserves these imports.

## Package Script Rewrite Rules

Migration rewrites commands provided by the Vite+ toolchain while preserving
their arguments: `vite` to `vp dev` or the matching `vp` subcommand, `vitest` to
`vp test`, `oxlint` to `vp lint`, `oxfmt` to `vp fmt`, `tsdown` to `vp pack`, and
`lint-staged` to `vp staged`. When their optional migrations run, `eslint` and
`prettier` are similarly rewritten to `vp lint` and `vp fmt`.

For commands launched through `bunx`, migration preserves `bunx` and its
`--bun` flag (keeping the user's chosen runtime) and rewrites only the managed
command. This also works when `bunx` follows a command-launcher delimiter such
as `run` or `--`:

| Before                                                  | After                                                    |
| ------------------------------------------------------- | -------------------------------------------------------- |
| `bunx --bun vite build`                                 | `bunx --bun vp build`                                    |
| `bunx --bun vitest run`                                 | `bunx --bun vp test run`                                 |
| `portless --tailscale run bunx --bun vite`              | `portless --tailscale run bunx --bun vp dev`             |
| `dotenv -e .env.test -- bunx --bun oxlint --type-aware` | `dotenv -e .env.test -- bunx --bun vp lint --type-aware` |

Unrelated `bunx` commands and other package-executor forms remain unchanged.

## Node.js Version Rules

Migration normalizes the project's Node.js pins so package managers do not skip
the native binding's optional dependency:

- `.nvmrc` and Volta `volta.node` pins are converted to `.node-version` (the
  format Vite+ reads). An existing `.node-version` is kept. When `.nvmrc` is
  removed, any `actions/setup-node` `node-version-file: .nvmrc` reference in
  `.github/workflows/*.{yml,yaml}` and composite actions
  (`.github/actions/**/action.{yml,yaml}`) is repointed to `.node-version` so CI
  does not fail with "node version file ... does not exist".
- Each pin (`.node-version`, `devEngines.runtime`, and `engines.node`) is checked
  independently against the Vite+ supported range (`package.json#engines.node`),
  on its _floor_ (the lowest version it permits). Package managers evaluate the
  native binding's optional dependency against that floor, so a pin such as `>=24`
  or `24` overlaps the supported range yet its floor (`24.0.0`) is below the
  minimum (`>=24.11.0`); pnpm then skips the native package and the toolchain
  fails with "Cannot find native binding".
- A pin whose floor is below the supported minimum is raised when its major has a
  supported release: `.node-version` to the concrete latest release of that major
  (`24.3.0`, `24.2`, `24` → `24.18.0`); `devEngines.runtime` and `engines.node` to
  an open `>=<supported minimum>` range (`>=24`, `^24`, `24` → `>=24.11.0`) so they
  keep accepting newer releases.
- A floor-supported pin (`24.18.0`, `>=24.11.0`, `^22.18.0`), an alias (`lts/*`),
  or a major with no supported release is left unchanged.

## Package-Manager Rules

### pnpm

- pnpm 10.6.2+ uses `pnpm-workspace.yaml` as the single source for supported
  root settings. Migration moves recognized `package.json#pnpm` fields there,
  including overrides, peer rules, patch settings, package extensions,
  architecture and build policy, audit/update configuration, and configuration
  dependencies. It removes the `pnpm` object when it becomes empty and preserves
  unknown keys that may belong to other tooling.
- When both files define the same migrated pnpm setting, migration recursively
  merges object entries and retains unique array entries. Values from
  `package.json#pnpm` win at conflicting scalar leaves, while workspace-only
  sibling entries are preserved.
- Before pnpm 10.6.2, migration retains these settings in
  `package.json#pnpm`. General workspace-setting support started in pnpm 10.5.0,
  but overrides required 10.5.1 and `peerDependencyRules` required 10.6.2. pnpm
  11 no longer reads the legacy package.json settings.
- Migration keeps dependency references, default and named catalogs, overrides,
  and `peerDependencyRules` consistent.
- pnpm accepts the logical default catalog as either top-level `catalog` or
  `catalogs.default`, but not both. Migration preserves the existing form and
  never creates the other form beside it.
- When an existing named catalog already owns `vite-plus`, `vite`, or `vitest`,
  migration reuses that managed toolchain catalog for newly added dependencies
  and overrides. It creates a top-level default catalog only when no managed or
  default catalog can be reused.
- Each package that lists `vite-plus` in `dependencies` or `devDependencies`
  gets a direct `vite` dev dependency unless it already declares `vite` in a
  dependency field.
- Unrelated selector-shaped and object-valued overrides are preserved.

### npm

- Migration normalizes direct aliases before adding the matching override so
  npm does not fail with `EOVERRIDE`.
- When changing a real Vite installation to the core alias, remove stale Vite
  install and lockfile state before reinstalling.
- Add a top-level `vite` edge for opt-in browser-provider layouts when nested
  Vitest packages otherwise cannot resolve it.

### Yarn

- Vite+ does not support Plug'n'Play. Detect explicit and implicit PnP before
  migration and convert the project to `nodeLinker: node-modules`. Preserve all
  unrelated `.yarnrc.yml` settings. `--no-interactive` accepts the conversion;
  a process-level `YARN_NODE_LINKER=pnp` must be fixed by the caller.
- Catalog references and user hoisting settings are preserved.
- Migration avoids split Vitest copies under workspace hoisting isolation. It
  applies a package-level fix where possible and warns when the isolation
  cannot be changed safely.

### Bun

- Bun catalogs only resolve inside a workspace (a root `package.json` with a
  non-empty `workspaces`). In a bun workspace, preserve existing top-level or
  workspace catalog locations and named catalog references. A standalone
  (single-package) bun project keeps concrete specs and writes no catalog field,
  because `bun install` cannot resolve `catalog:` outside a workspace.
- Mirror the core alias as a direct `vite` dependency so Bun sees the peer
  provider before applying overrides.

After updating the manifests and package-manager configuration, migration
reinstalls dependencies once to refresh the lockfile. If installation fails,
migration reports the error and exits with a nonzero status. After a successful
migration, it runs `vp fmt` on files changed during migration, excluding paths
that were already dirty in the Git worktree. Oxfmt selects the supported
formats. Non-Git projects retain full-project formatting. Formatting is skipped
while the project still uses Prettier. A formatter failure is reported as a
warning so the migration result and manual formatting command remain available.

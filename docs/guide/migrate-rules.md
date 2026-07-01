# Migration Rules

This guide explains how `vp migrate` updates dependencies, source imports, and
package-manager configuration in existing Vite+ projects. See the
[migration guide](./migrate.md) for the complete command overview.

## Before You Migrate

1. Run `vp upgrade` so the global CLI has the latest migration rules. When a
   project's local `vite-plus` is older, migration delegates to the global CLI,
   so a stale local copy is not a blocker.
2. Upgrade the project to Vite 8+ and Vitest 4.1+ when necessary.
3. Run `vp migrate` from the workspace root. Use `--no-interactive` in
   automated environments.
4. Review every changed manifest, package-manager config, source rewrite, and
   generated lockfile.
5. Validate with `vp install`, `vp check`, `vp test`, and `vp build`.

Running the migration again after a successful migration should not produce
another diff.

## Upgrade vs. Full Setup

On a project that already depends on `vite-plus`, `vp migrate` upgrades the
toolchain version only (dependencies, package-manager configuration, and import
finalization). It does not touch project setup.

- Pass `--full` to also run the setup bucket: git hooks, editor config, agent
  files, ESLint and Prettier migration, framework shims, the tsconfig `baseUrl`
  fix, and the `.nvmrc`/Volta to `.node-version` conversion.
- The per-action flags `--hooks`, `--agent`, and `--editor` opt into a single
  setup action without `--full`.

When a default upgrade skips available setup actions, it prints a hint to run
`vp migrate --full`. Fresh (non Vite+) projects always run the full migration.

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

| Dependency                     | Migration rule                                                                                                                                                                                                                                               |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `vite-plus`                    | Add it where the package is migrated. Re-pin plain ranges to the current concrete target, directly or through a catalog. Preserve deliberate protocol pins.                                                                                                  |
| `vite`                         | Keep existing declarations. With pnpm, add a direct dev dependency to every package that depends on `vite-plus` and does not already declare `vite`. Point managed edges and the shared override to the matching `@voidzero-dev/vite-plus-core` target.      |
| `vitest`                       | Remove it in the common node-mode case because `vite-plus` provides it transitively. Keep or add an exact bundled version only in packages with direct Vitest requirements.                                                                                  |
| `@vitest/*`                    | Align lockstep packages that the project directly lists to the bundled Vitest version. Reference them through the toolchain catalog when the package manager supports catalogs, adding an entry for any that lack one; otherwise write the concrete version. |
| `@voidzero-dev/vite-plus-test` | Remove all dependency, override, resolution, and catalog aliases. Rewrite imports to the current `vite-plus/test*` surface.                                                                                                                                  |

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

Catalog handling:

- reference them through the toolchain catalog when the package manager supports
  catalogs, preserving an existing `catalog:` / `catalog:<name>` reference and
  adding a catalog entry for any that lack one;
- update each catalog entry to the bundled Vitest version; and
- use the concrete bundled version only when catalogs are unsupported (npm, a
  standalone bun project, or a pre-catalog pnpm/Yarn).

Do not align independently versioned or obsolete packages:

- `@vitest/eslint-plugin` has its own version line;
- `@vitest/coverage-c8` stopped at an older release and has no Vitest 4 version;
  and
- third-party `vitest-*` integrations keep their own compatible package
  versions, although their required Vitest peer may require direct provisioning.

The base `@vitest/browser` runtime and `@vitest/browser-preview` are bundled by
Vite+ and should be removed as direct dependencies. The Playwright and
WebdriverIO providers remain opt-in. In a catalog-capable project, a kept or
injected provider is referenced through the preferred toolchain catalog and
added at the bundled Vitest version. Otherwise, it writes the concrete bundled
version.
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

On an existing Vite+ project this conversion is part of the full setup bucket, so
it runs with `vp migrate --full`. Fresh migrations run it unconditionally.
Migration converts legacy Node.js version manager files to the `.node-version`
format Vite+ reads:

- `.nvmrc` and Volta `volta.node` pins are converted to `.node-version` (the
  format Vite+ reads). An existing `.node-version` is kept. When `.nvmrc` is
  removed, any `actions/setup-node` `node-version-file: .nvmrc` reference in
  `.github/workflows/*.{yml,yaml}` and composite actions
  (`.github/actions/**/action.{yml,yaml}`) is repointed to `.node-version` so CI
  does not fail with "node version file ... does not exist".

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
- Catalogs are a separate feature, supported from pnpm 9.5.0, independent of the
  workspace-settings boundary. So below 10.6.2, where overrides stay in
  `package.json#pnpm`, migration still rewrites the workspace catalog off stale
  wrapper aliases and keeps `catalog:` overrides as references rather than
  inlining them to concrete versions.
- Migration keeps dependency references, default and named catalogs, overrides,
  and `peerDependencyRules` consistent.
- pnpm accepts the logical default catalog as either top-level `catalog` or
  `catalogs.default`, but not both. Migration preserves the existing form and
  never creates the other form beside it.
- When an existing named catalog already owns `vite-plus`, `vite`, or `vitest`,
  migration reuses that managed toolchain catalog for newly added dependencies
  and overrides. It creates a top-level default catalog only when no managed or
  default catalog can be reused.
- Each package that declares `vite-plus` also gets a direct `vite` dev
  dependency (see [Vite and Overrides](#vite-and-overrides)).
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

Migration inspects each Vite config for Rolldown-incompatible patterns (such as
`manualChunks`) and reports any it finds as warnings, without changing the config.

After updating the manifests and package-manager configuration, migration
reinstalls dependencies once to refresh the lockfile. If installation fails,
migration reports the error and exits with a nonzero status. After a successful
migration, it runs `vp fmt` on files changed during migration, excluding paths
that were already dirty in the Git worktree. Oxfmt selects the supported
formats. Non-Git projects retain full-project formatting. Formatting is skipped
while the project still uses Prettier. A formatter failure is reported as a
warning so the migration result and manual formatting command remain available.

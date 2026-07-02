# Migration Rules

This reference describes exactly what `vp migrate` does to a project: how it
updates dependencies, rewrites source imports and package scripts, and adjusts
package-manager configuration. See the [migration guide](./migrate.md) for the
command overview and workflow.

Except for [Before You Migrate](#before-you-migrate), which lists steps you
take yourself, everything below describes automatic behavior.

## Before You Migrate

1. Run `vp upgrade` so the global CLI has the latest migration rules. A stale
   local `vite-plus` is not a blocker: when the project's local copy is older,
   migration delegates to the global CLI.
2. Upgrade the project to Vite 8+ and Vitest 4.1+ when necessary.
3. Run `vp migrate` from the workspace root. Use `--no-interactive` in
   automated environments.
4. Review every changed manifest, package-manager config, source rewrite, and
   generated lockfile.
5. Validate with `vp install`, `vp check`, `vp test`, and `vp build`.

Migration is idempotent: running it again after a successful migration should
not produce another diff.

## Upgrade vs. Full Setup

On a project that already depends on `vite-plus`, `vp migrate` performs an
upgrade only: it updates dependencies and package-manager configuration and
finalizes imports. It does not touch project setup.

- `--full` also runs the setup actions: git hooks, editor config, agent files,
  ESLint and Prettier migration, framework shims, the tsconfig `baseUrl` fix,
  and the `.nvmrc`/Volta to `.node-version` conversion.
- `--hooks`, `--agent`, and `--editor` opt into a single setup action without
  `--full`.

When a default upgrade skips setup actions that would apply, it prints a hint
to run `vp migrate --full`. Fresh (non Vite+) projects always run the full
migration.

## Dependency Rules

What happens to each toolchain dependency, at a glance:

| Dependency                     | What happens                                                                                                                                                            |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vite-plus`                    | Added where the package is migrated; plain ranges re-pinned to the concrete target, directly or through a catalog.                                                      |
| `vite`                         | Existing declarations kept and pointed at the core alias. Under pnpm, added as a direct dev dependency wherever needed (see [Vite and Overrides](#vite-and-overrides)). |
| `vitest`                       | Removed in the common node-mode case because `vite-plus` provides it transitively. Kept or added only when [directly required](#when-vitest-is-directly-required).      |
| `@vitest/*`                    | Directly installed lockstep packages aligned to the bundled Vitest version (see [Vitest Ecosystem Packages](#vitest-ecosystem-packages)).                               |
| `@voidzero-dev/vite-plus-test` | Removed everywhere: dependencies, overrides, resolutions, and catalog aliases. Imports are rewritten to the current `vite-plus/test*` surface.                          |

### Version Selection

- `vite-plus` is pinned to the concrete version of the CLI running the
  migration, never the `latest` dist-tag.
- The `vite` alias targets `@voidzero-dev/vite-plus-core` from the same Vite+
  release.
- A catalog-backed manifest may contain `catalog:` or a named catalog
  reference. Migration keeps the reference and updates the referenced catalog
  value to the concrete toolchain target.
- Deliberate protocol pins are preserved: `workspace:`, `file:`, `link:`,
  `npm:`, `github:`, Git URLs, and HTTP URLs.
- Migration reconciles every workspace package, not only the root manifest.
  Shared overrides and catalogs stay at the workspace root; dependencies that
  provide a peer belong in each package that needs them.

### Vite and Overrides

Package-manager overrides do not create dependency edges by themselves. Under
pnpm, a package that lists `vite-plus` in `dependencies` or `devDependencies`
but has no `vite` entry anywhere (`dependencies`, `devDependencies`,
`optionalDependencies`, or `peerDependencies`) lets pnpm auto-install upstream
Vite to satisfy Vitest's required `vite` peer, splitting the project across
separate Vite+, Vite, and Vitest instances. To prevent this, `vp migrate` adds
the missing `vite` entry to `devDependencies` of every such package; the
workspace override then redirects it to Vite+ core.

Related rules:

- A direct `vite` declaration is never removed merely because a root override
  exists.
- Plain or stale aliases are normalized; named catalog references are kept.
- The direct-entry rule above is pnpm-specific. Bun mirrors its core alias as
  a direct dependency for its peer resolver, and npm browser-provider layouts
  may need a top-level `vite` edge so nested Vitest packages can resolve
  `vite`.

### When Vitest Is Directly Required

Migration keeps or adds a package-local `vitest` at the exact bundled version
when any of the following is true:

- an installed dependency has a non-optional `vitest` peer, whether exact or a
  range;
- the package uses Vitest browser mode or an opt-in browser provider;
- source or TypeScript configuration retains an upstream `vitest` reference;
- the package declares `@nuxt/test-utils`; or
- dependency metadata is unavailable and an existing direct `vitest` might be
  satisfying an unknown required peer.

Detection reads installed peer metadata, so integrations such as
`vite-plugin-gherkin` are handled even though their names do not contain
`vitest`.

When a package qualifies, migration:

- adds `vitest` to that package, not indiscriminately to every workspace
  package;
- uses the existing catalog reference when supported, otherwise the exact
  bundled version; and
- keeps a matching workspace override or resolution so the graph resolves a
  single Vitest version.

A peer declaration alone does not install Vitest. If a surviving
`peerDependencies.vitest` uses a catalog entry that migration will remove, it
is resolved to the public peer range first.

### Vitest Ecosystem Packages

Official current `@vitest/*` packages generally publish in lockstep with
Vitest. Migration aligns the ones the project directly installs, including
`@vitest/coverage-v8`, `@vitest/coverage-istanbul`, `@vitest/ui`, and
`@vitest/web-worker`:

- when the package manager supports catalogs, they are referenced through the
  toolchain catalog: an existing `catalog:` / `catalog:<name>` reference is
  preserved, a catalog entry is added for any package that lacks one, and each
  entry is updated to the bundled Vitest version;
- when catalogs are unsupported (npm, a standalone bun project, or a
  pre-catalog pnpm/Yarn), the concrete bundled version is written instead.

Packages that are **not** aligned:

- `@vitest/eslint-plugin` follows its own version line;
- `@vitest/coverage-c8` stopped at an older release and has no Vitest 4
  version; and
- third-party `vitest-*` integrations keep their own compatible versions,
  though their required Vitest peer may still trigger
  [direct provisioning](#when-vitest-is-directly-required).

For browser mode, the base `@vitest/browser` runtime and
`@vitest/browser-preview` are bundled by Vite+ and are removed as direct
dependencies. The Playwright and WebdriverIO providers stay opt-in: a kept or
injected provider is referenced through the preferred toolchain catalog at the
bundled Vitest version (or written concretely when catalogs are unsupported),
and its `playwright` or `webdriverio` peer is installed alongside.

Providers are detected before imports are rewritten. This covers legacy
projects that aliased `vitest` to `@voidzero-dev/vite-plus-test` and import
from `vitest/browser-<provider>`, `vitest/browser/providers/<provider>`, or
`vitest/plugins/browser-<provider>`: those imports still install the
corresponding `@vitest/browser-playwright` or `@vitest/browser-webdriverio`
dependency and its framework peer.

Object-valued nested npm and Bun overrides are preserved: they are
user-defined scopes rather than scalar version pins.

## Source Rewrite Rules

### `vite` Imports

`vite` and `vite/*` imports are rewritten to `vite-plus` **only in config
entry files**: `vite.config.*`, `vitest.config.*`, and any config file the
migration resolved. Every other file keeps its `vite` imports, for two
reasons:

- `vite-plus` is not a guaranteed superset of Vite's exposed surface. It owns
  only `defineConfig`, `defineProject`, and `lazyPlugins`, so rewriting a
  pass-through symbol such as `createBuilder` or `loadConfigFromFile`
  (including in `typeof import('vite')` type positions) can break.
- An unrewritten `vite` import still resolves through the
  `@voidzero-dev/vite-plus-core` alias in a Vite+ project.

Plugin packages (an unscoped name starting with `vite-plugin-` or
`unplugin-`, or `vite` in `peerDependencies`/`dependencies`) skip the rewrite
even in config files. Only the `vite` specifier is in scope for this rule.

### `vitest` and Browser Imports

- Ordinary `vitest` and `vitest/*` imports are rewritten to
  `vite-plus/test*`.
- Legacy Playwright and WebdriverIO provider imports are detected before this
  rewrite so their optional provider dependencies are not lost.
- Scoped `@vitest/browser*` imports are rewritten to the corresponding
  `vite-plus/test/browser*` exports, provisioning opt-in providers when
  needed.
- Existing `vite-plus/test*` imports are left unchanged.

### What Is Never Rewritten

- `declare module 'vitest'` and `declare module '@vitest/browser*'`: module
  augmentation must retain the upstream module identity.
- References that stay behind, such as `compilerOptions.types`,
  `require.resolve`, `import.meta.resolve`, and `vitest/package.json`,
  require package-local Vitest (see
  [When Vitest Is Directly Required](#when-vitest-is-directly-required)).
- In a package that declares `@nuxt/test-utils`, every `vitest` and
  `vitest/*` module specifier is preserved package-wide: the Nuxt transform
  requires the upstream identity and can otherwise inject a duplicate `vi`
  import. This exception does not apply to sibling packages or to scoped
  `@vitest/browser*` imports.

The `prefer-vite-plus-imports` lint rule follows the same Nuxt exception, so
lint autofix preserves these imports too.

## Package Script Rewrite Rules

Migration rewrites commands provided by the Vite+ toolchain in `package.json`
scripts while preserving their arguments:

| Before        | After                                       |
| ------------- | ------------------------------------------- |
| `vite`        | `vp dev`, or the matching `vp` subcommand   |
| `vitest`      | `vp test`                                   |
| `oxlint`      | `vp lint`                                   |
| `oxfmt`       | `vp fmt`                                    |
| `tsdown`      | `vp pack`                                   |
| `lint-staged` | `vp staged`                                 |
| `eslint`      | `vp lint`, when its optional migration runs |
| `prettier`    | `vp fmt`, when its optional migration runs  |

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

Migration converts legacy Node.js version-manager files to `.node-version`,
the format Vite+ reads. On an existing Vite+ project this conversion is part
of the full setup bucket, so it runs with `vp migrate --full`; fresh
migrations run it unconditionally.

- `.nvmrc` and Volta `volta.node` pins are converted to `.node-version`. An
  existing `.node-version` is kept.
- When `.nvmrc` is removed, any `actions/setup-node` `node-version-file:
.nvmrc` reference in `.github/workflows/*.{yml,yaml}` and composite actions
  (`.github/actions/**/action.{yml,yaml}`) is repointed to `.node-version` so
  CI does not fail with "node version file ... does not exist".

## Package-Manager Rules

### pnpm

**Root settings location.** pnpm 10.6.2+ uses `pnpm-workspace.yaml` as the
single source for supported root settings. Migration moves recognized
`package.json#pnpm` fields there, including overrides, peer rules, patch
settings, package extensions, architecture and build policy, audit/update
configuration, and configuration dependencies. It removes the `pnpm` object
when it becomes empty and preserves unknown keys that may belong to other
tooling.

- When both files define the same migrated setting, object entries are merged
  recursively and unique array entries are retained. Values from
  `package.json#pnpm` win at conflicting scalar leaves, while workspace-only
  sibling entries are preserved.
- Before pnpm 10.6.2, these settings stay in `package.json#pnpm`. (Workspace
  settings support arrived incrementally: 10.5.0 in general, 10.5.1 for
  overrides, 10.6.2 for `peerDependencyRules`. pnpm 11 no longer reads the
  legacy `package.json` settings.)

**Catalogs.** Catalogs are a separate feature, supported from pnpm 9.5.0,
independent of the settings boundary above. Even below 10.6.2, where
overrides stay in `package.json#pnpm`, migration still rewrites the workspace
catalog off stale wrapper aliases and keeps `catalog:` overrides as
references rather than inlining them to concrete versions.

- Dependency references, default and named catalogs, overrides, and
  `peerDependencyRules` are kept consistent with each other.
- pnpm accepts the logical default catalog as either top-level `catalog` or
  `catalogs.default`, but not both. Migration preserves the existing form and
  never creates the other form beside it.
- When an existing named catalog already owns `vite-plus`, `vite`, or
  `vitest`, migration reuses that managed toolchain catalog for newly added
  dependencies and overrides. It creates a top-level default catalog only
  when no managed or default catalog can be reused.

**Other rules.**

- Each package that declares `vite-plus` also gets a direct `vite` dev
  dependency (see [Vite and Overrides](#vite-and-overrides)).
- Unrelated selector-shaped and object-valued overrides are preserved.

### npm

- Direct aliases are normalized before the matching override is added, so npm
  does not fail with `EOVERRIDE`.
- When a real Vite installation changes to the core alias, stale Vite install
  and lockfile state is removed before reinstalling.
- Opt-in browser-provider layouts get a top-level `vite` edge when nested
  Vitest packages otherwise cannot resolve it.

### Yarn

- Vite+ does not support Plug'n'Play. Migration detects explicit and implicit
  PnP and converts the project to `nodeLinker: node-modules`, preserving all
  unrelated `.yarnrc.yml` settings. `--no-interactive` accepts the
  conversion; a process-level `YARN_NODE_LINKER=pnp` must be fixed by the
  caller.
- Catalog references and user hoisting settings are preserved.
- Migration avoids split Vitest copies under workspace hoisting isolation: it
  applies a package-level fix where possible and warns when the isolation
  cannot be changed safely.

### Bun

- Bun catalogs only resolve inside a workspace (a root `package.json` with a
  non-empty `workspaces`). In a bun workspace, existing top-level or
  workspace catalog locations and named catalog references are preserved. A
  standalone (single-package) bun project keeps concrete specs and gets no
  catalog field, because `bun install` cannot resolve `catalog:` outside a
  workspace.
- The core alias is mirrored as a direct `vite` dependency so Bun sees the
  peer provider before applying overrides.

## After the Migration

- Each Vite config is inspected for Rolldown-incompatible patterns (such as
  `manualChunks`). Anything found is reported as a warning; the config is not
  changed.
- Dependencies are reinstalled once to refresh the lockfile. If installation
  fails, migration reports the error and exits with a nonzero status.
- After a successful migration, `vp fmt` runs on the files changed during
  migration, excluding paths that were already dirty in the Git worktree.
  Oxfmt selects the supported formats; non-Git projects retain full-project
  formatting. Formatting is skipped while the project still uses Prettier. A
  formatter failure is reported as a warning so the migration result and the
  manual formatting command remain available.

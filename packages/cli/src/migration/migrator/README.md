# `migrator/` — migration logic, split by category

The `vp migrate` implementation used to live in a single ~7,300-line
`packages/cli/src/migration/migrator.ts`. It is now split into category
modules in this directory. `migration/migrator.ts` is a **barrel** that only
re-exports them:

```ts
// migration/migrator.ts
export * from './migrator/shared.ts';
export * from './migrator/eslint.ts';
// ...one line per module
```

So **external code keeps importing from `./migrator.ts`** (the barrel) and
nothing outside this directory had to change.

## Modules

Pick the file by what a function _does_, not by where it happens to be called.

| File                     | Owns                                                                                                                                                                                                                    |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `orchestrators.ts`       | Top-level entry points that wire everything together: `rewriteStandaloneProject`, `rewriteMonorepo`, `rewriteMonorepoProject`.                                                                                          |
| `package-json.ts`        | `rewritePackageJson` and the direct dependency-spec rewriting it does.                                                                                                                                                  |
| `vite-plus-bootstrap.ts` | "Already on Vite+" detection and the bootstrap/reconcile path: `detectVitePlusBootstrapPending`, `ensureVitePlusBootstrap`, `reconcileVitePlusBootstrapPackage`, the `ensure*`/`*Pending`/`*SatisfiesVitePlus` helpers. |
| `catalog.ts`             | `pnpm-workspace.yaml` / catalog / overrides / build-allowance writers and catalog-dependency resolvers; bun catalog; `rewriteRootWorkspacePackageJson`; pnpm workspace-settings migration.                              |
| `vitest-ecosystem.ts`    | Detecting direct vitest usage, the override-key ("dependency selector") parsing/dropping logic, managed-override sets, ecosystem alignment, legacy `vite-plus-test` wrapper-alias pruning.                              |
| `yarn.ts`                | `.yarnrc.yml`, Yarn PnP detection, workspace-hoisting fix, webdriverio detection.                                                                                                                                       |
| `source-scan.ts`         | Scanning a project's source tree for signals (browser-mode, opt-in providers, `@nuxt/test-utils`, retained upstream-vitest references).                                                                                 |
| `vite-config.ts`         | `vite.config.ts` merging, default-config injection, staged-config merge, lazy-plugin wrapping, import rewriting (`rewriteAllImports`), migrated-oxlint-config sanitization, lint-staged removal.                        |
| `eslint.ts`              | ESLint → Oxlint migration, oxlint JS-plugin namespace handling, ESLint prompts/warnings.                                                                                                                                |
| `prettier.ts`            | Prettier → Oxfmt migration and its prompts/warnings.                                                                                                                                                                    |
| `tsconfig.ts`            | `tsconfig.json` cleanup and `types` rewriting.                                                                                                                                                                          |
| `framework-shim.ts`      | Framework (Vue/Astro) shim detection and injection.                                                                                                                                                                     |
| `git-hooks.ts`           | husky / lint-staged → `vp staged` hook migration.                                                                                                                                                                       |
| `setup.ts`               | `packageManager` pin and Node version-manager file (`.nvmrc`/`.node-version`/Volta) migration.                                                                                                                          |
| `core-finalization.ts`   | Finalizing an existing-Vite+ core migration (rules YAML, core package scripts).                                                                                                                                         |
| `shared.ts`              | Cross-cutting constants, types, and tiny utilities used by **two or more** modules. The only module that other modules import from directly (see rules).                                                                |

## Rules for adding / changing code

1. **Add a function to the module that matches its category.** Keep it
   `export`ed so the barrel surfaces it. If nothing fits, prefer extending an
   existing module over creating a new one; if you do add a module, add a line
   for it to `migration/migrator.ts`.

2. **Importing another module's helper — use the barrel.** Reference
   cross-module **functions** via the barrel:

   ```ts
   import { managedOverridePackages, rewritePackageJson } from '../migrator.ts';
   ```

   This creates an import cycle (the barrel re-exports your module), which is
   **safe only because** these helpers are referenced _inside function bodies_
   (at runtime), never at module-evaluation time. Preserve that invariant: do
   not call a cross-module helper at the top level of a module.

3. **`shared.ts` is the one exception — import it directly, never via the
   barrel.** Things in `shared.ts` (e.g. `REMOVE_PACKAGES`,
   `OPT_IN_BROWSER_PROVIDERS`, shared types) are referenced at _module-load_
   time, so they must be imported from a fully-evaluated leaf:

   ```ts
   import { REMOVE_PACKAGES, type CatalogDependencyResolver } from './shared.ts';
   ```

   Keep `shared.ts` a **pure leaf**: it may import external packages and
   `../utils/*` / `./report.ts` etc., but it must **not** import from any
   sibling module or the barrel.

4. **Where does a new shared thing go?** A constant / type / helper used by a
   single module lives in that module. The moment a second module needs it,
   move it to `shared.ts` and `export` it.

5. **This split is structure only.** The barrel contains no logic; behavior
   changes belong in the relevant module and need their unit test (and snap
   test) updated, not the barrel.

## When to add a new module

Default to extending an existing module. Add a new file only when one of these
holds:

- **A new self-contained category appears** — usually a new tool/format being
  migrated (mirrors `eslint.ts`, `prettier.ts`, `yarn.ts`, `git-hooks.ts`).
  Test: you can name its single responsibility without saying "and".
- **An existing module outgrows readability _and_ has a clean seam** — a
  cohesive, loosely-coupled sub-cluster. Rough trigger: **>~900–1,000 lines plus
  a natural split point** (e.g. `catalog.ts` could become `catalog.ts` +
  `pnpm-workspace.ts`). Size alone is not enough.
- **Scattered helpers form a coherent theme** and consolidating them aids
  discoverability.

Do **not** add a file when: the function fits an existing category (extend it);
it's one small helper (owning module, or `shared.ts` if shared); it would be 1–2
functions with no theme (fragmentation is as bad as a monolith); or it would
have a tight two-way dependency with another module (they belong together).

When you do add one: give it a single-responsibility name, add its `export *`
line to `migration/migrator.ts`, add a row to the module table above, and decide
its layer — if siblings reference it at **load time** it must be a pure leaf (or
those pieces go in `shared.ts`); helpers used only inside function bodies may
import from the barrel.

## Validating a change

From the repo root:

```bash
vp check                                             # format + lint + type-check
pnpm -F vite-plus exec vitest run src/migration      # migration unit tests
just snapshot-test migration                         # migration CLI snapshot tests
```

Review any changed `.md` snapshots like code. A pure reorganization must not
change type-check results, the unit-test count, or snapshots. A behavior change
should be accompanied by the matching unit or snapshot test updates.

# RFC: Migrating Existing Vite+ Projects to a New Version

- Status: Partially implemented on `rfc/migrate-upgrade-path` (commits `03689668`, `3e5a5137`); vitest-removal simplification and browser-mode verification pending (see Follow-ups)
- Depends on: [#1588 replace @voidzero-dev/vite-plus-test with upstream vitest](https://github.com/voidzero-dev/vite-plus/pull/1588) (merged, `342fd2f4`)
- Related: `docs/guide/upgrade.md`, [migration-command.md](./migration-command.md), [upgrade-command.md](./upgrade-command.md)

## Goal: upgrade in two commands

Any later Vite+ upgrade is two commands: upgrade the global CLI, then migrate the project.

```bash
vp upgrade   # update the global `vp` binary
vp migrate   # bring the project up to the new toolchain
```

Both are needed, and the order matters. `vp migrate` normally runs the project's **local** `vite-plus`, which on an old project predates the new upgrade logic (and would even rewrite config that pins the project to the old version). So `vp upgrade` first makes a new-enough CLI available, and `vp migrate` then escalates to it (see Routing) and applies the rules below. `vp update vite-plus` alone is not enough: it bumps the dependency but does not reconcile the override/catalog config.

`vp migrate` is idempotent: on an already-current project it reports "already using Vite+" and changes nothing.

## Migrate rules

Run on an existing Vite+ project, in order. The guiding fact for vitest: `vite-plus` declares `vitest` (and the `@vitest/*` runtime family) as dependencies at the bundled version, so a project never needs its own `vitest`. It resolves transitively, and an ecosystem package resolves its exact `vitest` peer against it. Verified on `node-modules/urllib` across pnpm, npm, and yarn (PRs [#832](https://github.com/node-modules/urllib/pull/832) / [#833](https://github.com/node-modules/urllib/pull/833) / [#834](https://github.com/node-modules/urllib/pull/834)): with the direct `vitest` removed, coverage stays green on all three. The complementary forced-single case (a third-party `vitest-browser-svelte` keeps a managed `vitest`) is covered by the `migration-vitest-peer-dep` snap test.

| Area | Rule |
| ---- | ---- |
| Routing | If the project's local `vite-plus` is older than the global `vp`, run `migrate` from the global CLI; otherwise keep local-first. |
| `vite-plus` spec | Re-pin a non-protocol-pinned spec (e.g. `^0.1.24`) to the toolchain target (`catalog:` in catalog projects, else the version) so the lockfile moves off the old resolution. Preserve deliberate protocol pins (`workspace:`/`file:`/`link:`/`npm:`/...). |
| `vite` override | Always managed: alias `vite` to `npm:@voidzero-dev/vite-plus-core@latest` in whatever override/resolution/catalog form the project uses; normalize a behind `core@<old>` alias. |
| `vitest` itself (default) | Provided by `vite-plus`, so by default not project-managed: remove any project-level `vitest` from dependency fields, `overrides`/`resolutions`/`pnpm.overrides`, `pnpm-workspace.yaml` `overrides`+`catalog(s)`, bun/yarn catalog, and the `vitest` entry in pnpm `peerDependencyRules`. A future `vp update vite-plus` then keeps it correct with no project pin to drift. |
| `vitest`, forced-single exception | Keep a managed `vitest` (add to `devDependencies` **and** override/pin it to the bundled version) when the project has a **non-exact** `vitest` peer to collapse: a third-party integration on a range peer (`vitest-browser-react` / `-vue` / `-svelte`, ...), vitest browser mode, or a direct `vitest` source import. The override forces the range down to `vite-plus`'s exact version (one copy); the `devDependencies` entry satisfies the peer deterministically. Official `@vitest/*` (exact peer) do NOT trigger this, their exact peer already dedupes to `vite-plus`'s vitest. |
| `vitest` ecosystem packages | Align every official `@vitest/*` package the project lists (`@vitest/coverage-v8`, `@vitest/coverage-istanbul`, `@vitest/ui`, `@vitest/web-worker`, ...) to the bundled `VITEST_VERSION`, since each carries an **exact** `vitest` peer. Exclude `@vitest/eslint-plugin` (separate version line, `vitest: *` peer). Browser packages keep their dedicated handling: `@vitest/browser` / `-preview` are bundled by `vite-plus`; `@vitest/browser-playwright` / `-webdriverio` are opt-in (pinned + framework peer kept). |
| Legacy wrapper | Remove every `@voidzero-dev/vite-plus-test` alias (deps, overrides, catalogs); repoint direct wrapper imports to `vite-plus/test`. `vite-plus/test*` imports are left unchanged (stable public API). |
| pnpm config location | An empty `"pnpm": {}` with an existing `pnpm-workspace.yaml` reconciles the workspace file (instead of writing a second, conflicting override block into `package.json`). |
| Reinstall + verify | One reinstall with lockfile refresh (`--no-frozen-lockfile` / `--force`); a failed install warns and sets a non-zero exit. |

Force-override/CI mode (`VP_OVERRIDE_PACKAGES`) is respected: when `vitest` is not a managed key there, the project's own `vitest` is never stripped.

**Pending verification:** vitest **browser mode** historically needed a direct `vitest` injected (the "vibe-dashboard" regression). That predates `vite-plus` declaring `vitest`+`@vitest/browser` as dependencies and may now be obsolete, but it is not yet confirmed across package managers, so the browser-mode injection stays until a urllib-style 3-PM check clears it.

## Vitest ecosystem packages

How each package the `vitest` ecosystem rule covers is handled, verified against the registry at `4.1.9`. The code rule: align any `@vitest/*` the project lists to `VITEST_VERSION`, except `@vitest/eslint-plugin`; the browser packages additionally follow their bundled/opt-in handling.

| Package | `vitest` peer | Handling |
| ------- | ------------- | -------- |
| `@vitest/coverage-v8` | `4.1.9` (exact) | align to `VITEST_VERSION` |
| `@vitest/coverage-istanbul` | `4.1.9` | align to `VITEST_VERSION` |
| `@vitest/ui` | `4.1.9` | align to `VITEST_VERSION` |
| `@vitest/web-worker` | `4.1.9` | align to `VITEST_VERSION` |
| `@vitest/browser` | `4.1.9` | removed (bundled by `vite-plus`) |
| `@vitest/browser-preview` | `4.1.9` | removed (bundled by `vite-plus`) |
| `@vitest/browser-playwright` | `4.1.9` + `playwright` | opt-in: pin to `VITEST_VERSION`, keep `playwright` peer |
| `@vitest/browser-webdriverio` | `4.1.9` + `webdriverio` | opt-in: pin to `VITEST_VERSION`, keep `webdriverio` peer |
| `@vitest/expect` `/runner` `/snapshot` `/spy` `/utils` `/mocker` `/pretty-format` | none | transitive deps of `vitest`; `vite-plus` provides them, the project does not list them |
| `@vitest/eslint-plugin` | `*` | left as-is (own version line, e.g. `1.6.x`) |
| `vitest-browser-react` `/-vue` `/-svelte`, ... | `^4` (range) | third-party, own versioning; left at a compatible release, **and** a managed `vitest` is kept (devDep + override) to force a single copy against the range peer |

## Implementation

| Area | Change |
| ---- | ------ |
| `crates/vite_global_cli` (`commands/migrate.rs`, `js_executor.rs`) | `delegate_migrate`: compare local `vite-plus` vs global `vp` version; escalate to the global CLI when older. |
| `packages/cli/src/migration/migrator.ts` | Managed override set (`managedOverridePackages`); `vitest` removal across every sink; coverage-provider alignment; behind `vite-plus`/`vite` re-pin; empty-`pnpm` routing fix. |

Covered by unit tests in `migrator.spec.ts` (vitest removal, coverage alignment, behind re-pin, empty-`pnpm` reconciliation) and a routing test in `vite_global_cli`.

Not yet reflected in code: the current implementation still *pins* `vitest` when the project lists a vitest ecosystem package, rather than removing it. The "vitest itself: never project-managed" rule above (validated by the urllib 3-PM PRs) makes that pin unnecessary; collapsing it into unconditional removal is the next code change.

## Follow-ups (not in this change)

- Refine the code so `vitest` is removed even when a vitest ecosystem package is present (keep only the ecosystem-package alignment), per the validated rule.
- Verify vitest browser mode across pnpm/npm/yarn with no direct `vitest`; remove the browser-mode injection if it is obsolete.
- Regenerate `snap-tests-global/migration-*` and add an end-to-end check on a real `0.1.x` project.
- Update `docs/guide/upgrade.md` / the release-notes prompt to the `vp upgrade && vp migrate` flow once shipped, and `npm deprecate @voidzero-dev/vite-plus-test`.
- Optional `vp migrate --check` (detection-only, exit code signals an available upgrade) for CI.

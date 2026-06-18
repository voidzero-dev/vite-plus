# RFC: `vp migrate` Upgrade Path for Existing Vite+ Projects

- Status: Draft (for discussion)
- Depends on: [#1588 refactor: replace @voidzero-dev/vite-plus-test with upstream vitest](https://github.com/voidzero-dev/vite-plus/pull/1588) (merged, `342fd2f4`)
- Spec source: the ["Upgrading from 0.1.x to 0.2.1 Prompt"](https://github.com/voidzero-dev/vite-plus/releases/tag/v0.2.1) in the v0.2.1 release notes
- Related: [migration-command.md](./migration-command.md), [upgrade-command.md](./upgrade-command.md), `docs/guide/upgrade.md`

## Summary

The v0.2.1 release notes ship a careful, manual AI-agent prompt for upgrading a project from v0.1.x and explicitly say:

> Do not run `vp migrate` for this upgrade; it is not reliable enough yet. Make the changes yourself by editing the project's files, then verify by running the tools.

That prompt is the authoritative description of the correct end state. This RFC's goal is to make `vp migrate` reliably reproduce that end state so the disclaimer can be removed. The prompt also corrects a key assumption in earlier drafts of this RFC: the upgrade is NOT "always pin `vitest`". It is a usage-based decision that, in the common case, REMOVES `vitest` from the project entirely and lets it arrive transitively through `vite-plus`.

## Background

PR #1588 (shipped in v0.2.0) deleted the bundled `@voidzero-dev/vite-plus-test` wrapper and consumes upstream `vitest` directly. Today `ensureVitePlusBootstrap` (`migrator.ts`) unconditionally writes a managed `vitest` entry (pinned to `VITEST_VERSION`, currently `4.1.9`) into the project's override/catalog block for every already-Vite+ project, alongside the `vite` -> `npm:@voidzero-dev/vite-plus-core@latest` alias. `@vitest/*` runtime internals are NOT pinned (they are exact deps of `vitest`); coverage providers (`@vitest/coverage-v8` / `-istanbul`) are NOT managed and only get a runtime skew guard in `define-config.ts`.

### What #1588 already handles

PR #1588 added an "existing Vite+ project" repair path: `detectVitePlusBootstrapPending` + `ensureVitePlusBootstrap`, wired into the "already using Vite+" branch of `bin.ts` with one reinstall via `handleInstallResult`. It rewrites a stale `vitest: npm:@voidzero-dev/vite-plus-test@*` wrapper alias to the bundled vitest, proven by the `migration-already-vite-plus` snap fixture (even under `--no-interactive`). This is the foundation to build on, but as the prompt and the urllib evidence below show, it does the wrong thing in two ways: it pins `vitest` even when the project does not use it, and it misses several stale shapes.

### The real gap: upgrading a v0.1.x project (urllib)

`vp migrate` on a real 0.1.24 project (`node-modules/urllib`) did NOT upgrade. Its `package.json`:

```jsonc
{
  "devDependencies": {
    "@vitest/coverage-v8": "^4.1.8",
    "vite": "npm:@voidzero-dev/vite-plus-core@^0.1.24",
    "vite-plus": "^0.1.24",
    "vitest": "npm:@voidzero-dev/vite-plus-test@^0.1.24"
  },
  "overrides": {},
  "pnpm": {},
  "packageManager": "pnpm@11.7.0"
}
```

plus a committed `pnpm-workspace.yaml` written by the old CLI that actively pins the stack to 0.1.x:

```yaml
overrides:
  vite: 'npm:@voidzero-dev/vite-plus-core@^0.1.24'    # forces core to 0.1.x
  vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24'  # forces the deleted wrapper
```

Observed blockers, each sufficient on its own:

1. **Routing: the stale local CLI runs.** `vp migrate` delegates **local-first** (`crates/vite_global_cli/src/commands/delegate.rs`). urllib has `vite-plus@0.1.24` installed, so the global `vp v0.2.x` delegated to the **0.1.24** CLI, which predates #1588 and has no upgrade logic. Worse, that old CLI rewrites the old-shape `pnpm-workspace.yaml`, pinning `vite`/`vitest` to `^0.1.24` and the dead wrapper, which then blocks any later upgrade. The documented `vp update vite-plus --latest && vp migrate` flow does not escape this, because `vp update` deliberately does not reconcile those pins (`docs/guide/upgrade.md`).

2. **The v0.1.x shapes are not repaired by the v0.2.x bootstrap.** `ensureVitePlusDependencySpecs` only re-pins `vite-plus` when its spec is `catalog:` or absent, so a pinned `^0.1.24` is left untouched and never reaches the target. The inline `vite`/`vitest` aliases in `devDependencies` are never rewritten. The `vite` override is a **behind core alias** (`core@^0.1.24`), not the dead wrapper, so the wrapper-only `pruneLegacyWrapperAliases` does not normalize it.

3. **Empty `"pnpm": {}` misroutes the repair.** Both `detectVitePlusBootstrapPending` and `ensureVitePlusBootstrap` branch on `if (pkg.pnpm)`, and `{}` is truthy, so they inspect `pkg.pnpm.overrides` (empty) and take the `if (!pkg.pnpm)` -> false path that **skips the `pnpm-workspace.yaml` rewrite entirely**. A fresh override block lands in `package.json` while the pinning overrides in `pnpm-workspace.yaml` survive, leaving two conflicting override sources. This is effectively a standalone bug in #1588.

### What the v0.2.1 prompt specifies (the correct end state)

The prompt encodes the upgrade as these steps (paraphrased; see the release for verbatim text):

1. **Set `vite-plus` to the exact target version (`0.2.1`) and reinstall**, in every workspace package that depends on it. "Changing the spec to `0.2.1` is what moves the lockfile off the old resolution; a reinstall that leaves the spec unchanged would keep the old version." Exact, not a range or `latest`.
2. **Remove the `@voidzero-dev/vite-plus-test` wrapper everywhere** (package.json, lockfile, pnpm-workspace.yaml / .yarnrc.yml catalogs, source imports). Then a **usage-based decision**:
   - The project depends on vitest directly ONLY IF a source/test file imports from `vitest` or `@vitest/...`, OR a `@vitest/*` package is in its deps (e.g. a coverage provider). Imports from `vite-plus/test` do NOT count.
   - **Common case (no direct usage): remove vitest configuration entirely.** Delete the `vitest` entry from dependencies in whatever form (wrapper alias, `catalog:`, plain version), and remove `vitest` from every resolution mechanism (`overrides`, `resolutions`, pnpm `overrides`/`catalog` in package.json or pnpm-workspace.yaml, any catalog). Do NOT add a pinned `vitest`; it arrives transitively through `vite-plus`.
   - **Direct-usage case: pin upstream vitest to the bundled version (`4.1.9`) and align the whole ecosystem.** Set every `@vitest/*` the project lists (`coverage-v8`, `ui`, `browser`, ...) to that same version, and update other integration packages (`vitest-browser-*`) to a compatible release. "Leaving an ecosystem package on an older version pulls in a second copy of vitest, which Vitest rejects at runtime."
   - Delete dependency-resolution config that existed only for the wrapper/old vitest: pnpm `peerDependencyRules` (`allowedVersions` / `ignoreMissing`) referencing `vitest` / `@vitest/*` / the wrapper, and yarn `packageExtensions` equivalents. Leave unrelated rules.
3. **Keep the `vite` -> core override, pinned to the exact target**: `vite` -> `npm:@voidzero-dev/vite-plus-core@0.2.1`, in whatever override/resolution/catalog form the project already uses. Core is released in lockstep with `vite-plus`.
4. **Leave `vite-plus/test*` imports unchanged**; only repoint direct `@voidzero-dev/vite-plus-test` imports to `vite-plus/test`.
5. **Reinstall and verify**: no `@voidzero-dev/vite-plus-test` references remain outside `node_modules`; the tree resolves to a **single** `vitest` version (no duplicates); tests pass (native Vitest banner); the `vp check` workflow passes.

Constraints: do not bypass git hooks (report pre-existing failures instead); make the smallest set of edits; end with a short summary.

Two insights from this change the design:

- **The common case is removal, not pinning.** Removing `vitest` (rather than pinning it to an exact version) is what lets future `vp update vite-plus` keep vitest correct automatically: there is no project-level pin to drift. urllib is NOT the common case (it lists `@vitest/coverage-v8`), so it takes the direct-usage branch: pin `vitest` to `4.1.9` and set `@vitest/coverage-v8` to `4.1.9`, which is exactly the version it was missing.
- **Exactness moves the lockfile.** The upgrade must write exact target versions for `vite-plus` and the core alias, in every workspace package, or the lockfile keeps resolving the old version.

## Goals

1. `vp migrate` reliably reproduces the v0.2.1 prompt's end state for a v0.1.x project, so the "do not run `vp migrate`" disclaimer can be dropped.
2. Run the upgrade with a CLI new enough to contain this logic (fix the local-first routing that runs a stale 0.1.x CLI).
3. Implement the usage-based vitest decision: remove vitest entirely in the common case; pin + align the ecosystem in the direct-usage case.
4. Pin `vite-plus` and the `vite`->core alias to the exact target version, in every workspace package, so the lockfile moves.
5. Repair all observed stale shapes: inline/behind aliases, the empty-`pnpm` misrouting, dual override sources, and wrapper-only peer config.
6. Verify the end state (no wrapper refs, single vitest version) and respect the prompt's constraints (git hooks, minimal edits, summary). Idempotent on re-run.

## Non-Goals

- Changing behavior for projects that do not yet use `vite-plus` (the full-migration path already writes the canonical shape).
- Rewriting user source beyond repointing direct `@voidzero-dev/vite-plus-test` imports; `vite-plus/test*` stays the stable public API.
- Pinning the `@vitest/*` runtime internals individually (they cascade from `vitest`). The ecosystem alignment in the direct-usage case targets the packages the project itself lists, not transitive internals.

## Design

### 1. Run the right vp (routing)

The upgrade logic must execute from a CLI at least as new as the target. The prompt's manual workaround is "after any install, re-resolve vp so you always run the version currently in the project." Automate the same idea:

- In the global CLI (`crates/vite_global_cli`), before delegating `migrate` local-first, read the local `vite-plus` version and compare to the global `vp`. If local is older, run `migrate` from the **global** JS CLI (`delegate_to_global_cli`) instead of the stale local one. The global CLI's constants (target version, `VITEST_VERSION`) are then self-consistent.
- The upgrade re-pins `vite-plus` to the global version and reinstalls, so the next `vp` in the project resolves to the upgraded local CLI.

This is mandatory, not polish: the stale local CLI does not just no-op, it writes pinning overrides that block the upgrade. Simpler alternative (Open Questions): always route `migrate` through the global CLI.

### 2. Bump `vite-plus` to the exact target, everywhere, and reinstall

For every workspace package that depends on `vite-plus`, set the spec to the exact executing-CLI version (e.g. `0.2.1`), not a range or `catalog:`/`latest` placeholder. Extend `ensureVitePlusDependencySpecs` to re-pin a concrete behind spec (`^0.1.24`), not only `catalog:`/absent. Then reinstall with lockfile refresh (`--no-frozen-lockfile` / `--force`) so the lockfile moves off the old resolution.

### 3. Remove the wrapper and apply the usage-based vitest decision

This replaces `ensureVitePlusBootstrap`'s unconditional "write `vitest` into overrides" with the prompt's logic:

1. **Detect direct vitest usage**: a source/test file imports from `vitest` or `@vitest/...` (not `vite-plus/test`), OR the project lists any `@vitest/*` package in a dependency field. (Source scan can reuse the migration's existing import walker.)
2. **Common case (no direct usage): purge vitest.** Remove the `vitest` dependency entry in any form, and remove `vitest` from every resolution mechanism (`overrides`, `resolutions`, `pnpm.overrides`, `pnpm-workspace.yaml` `overrides`/`catalog`, bun `workspaces.catalog`, yarn `resolutions`/`.yarnrc.yml` catalog). Add no pin.
3. **Direct-usage case: pin and align.** Set `vitest` (the dependency and/or override the project uses) to `VITEST_VERSION`, and set every `@vitest/*` package the project lists to the same version; bump `vitest-browser-*` and similar integration packages to a compatible release. This subsumes the earlier "coverage provider alignment" goal: `@vitest/coverage-v8: ^4.1.8` -> `4.1.9`.
4. **Behind/inline aliases**: rewrite `vite: npm:@voidzero-dev/vite-plus-core@<old>` to the exact target (`@0.2.1`) wherever it appears, including inline `devDependencies` aliases; reuse `pruneLegacyWrapperAliases` for the dead wrapper and add normalization for behind core aliases.

### 4. Pin the `vite` -> core override to the exact target

Keep the `vite` -> `npm:@voidzero-dev/vite-plus-core@<target>` mapping, set to the exact executing version, in whichever override/resolution/catalog form the project already uses. This is a deliberate change from the current `@latest` convention (see Open Questions) and matches the prompt's lockstep requirement.

### 5. Clean wrapper-only resolution config and fix the pnpm location

- Remove pnpm `peerDependencyRules` (`allowAny` / `allowedVersions` / `ignoreMissing`) and yarn `packageExtensions` entries that reference `vitest`, `@vitest/*`, or the wrapper, when they exist only to accommodate the old setup. Leave unrelated rules.
- Treat an empty/partial `pkg.pnpm` (e.g. `"pnpm": {}`) as "no package.json pnpm config" so the `pnpm-workspace.yaml` path runs. When both a `package.json` `pnpm.overrides` and a `pnpm-workspace.yaml` `overrides` exist, reconcile both so the project is not left with two conflicting override sources.

### 6. Reinstall and verify

After edits, reinstall once (reusing `handleInstallResult`), then assert the prompt's post-conditions and surface failures as warnings + non-zero exit:

- No `@voidzero-dev/vite-plus-test` reference anywhere outside `node_modules` (package.json, lockfile, catalogs, sources).
- The dependency tree resolves to a **single** `vitest` version (no duplicate copies). This is the check that catches a missed ecosystem package in the direct-usage branch.
- `vite-plus`, the core alias, and (if present) the aligned `@vitest/*` packages resolve to the expected versions.

### 7. Constraints and UX

Honor the prompt's constraints: do not bypass git hooks (if a pre-existing failure blocks the run, report it rather than forcing through); make the smallest set of edits and do not reformat unrelated files; end with a summary. Interactive run prompts once for the whole upgrade; `--no-interactive` applies it. Summary, fed by `MigrationReport`:

```
Upgraded Vite+ 0.1.24 -> 0.2.1
  re-pinned vite-plus and vite->core to 0.2.1 (1 package)
  removed @voidzero-dev/vite-plus-test wrapper
  project uses vitest directly (@vitest/coverage-v8): pinned vitest 4.1.9, aligned @vitest/coverage-v8 4.1.8 -> 4.1.9
  verified: no wrapper refs, single vitest version
```

### 8. Idempotency

After a successful upgrade, detection returns false (target version pinned, no wrapper, single vitest), so a re-run hits the "already using Vite+, happy coding" path. Repairs must be recoverable by re-running if an install fails after files were rewritten.

## Code Touchpoints

| Area | Change |
| ---- | ------ |
| `crates/vite_global_cli/src/commands/migrate.rs` (+ `delegate.rs`) | Local-vs-global version check; route `migrate` to the global CLI when local `vite-plus` is older |
| `packages/cli/src/migration/migrator.ts` | Replace unconditional vitest pinning with the usage-based decision; exact-version pin of `vite-plus` + core alias for every workspace package; behind/inline alias normalization; empty-`pnpm` fix and dual-source reconciliation; wrapper-only peer-config cleanup |
| `packages/cli/src/migration/detector.ts` | Detect direct vitest usage (source imports + listed `@vitest/*`) |
| `packages/cli/src/migration/bin.ts` | Drive the upgrade in the already-Vite+ path; verify single-vitest post-condition; summary |
| `packages/cli/src/migration/report.ts` | Report version bump, removal-vs-pin decision, ecosystem alignment, verification |
| `docs/guide/upgrade.md` / release notes | Replace the manual prompt + "do not run `vp migrate`" with `vp upgrade && vp migrate` once reliable |

## Testing Plan

- **Unit** (`migrator.spec.ts`):
  - urllib shape (pnpm, inline `vite`/`vitest` aliases, pinned `vite-plus: ^0.1.24`, empty `"pnpm": {}`, committed `pnpm-workspace.yaml` pinning to `^0.1.24`/wrapper, `@vitest/coverage-v8: ^4.1.8`) -> direct-usage branch: `vite-plus`/core pinned to target, `vitest` pinned `4.1.9`, `@vitest/coverage-v8` -> `4.1.9`, no wrapper, single override source.
  - Common-case shape (uses only `vite-plus/test`, no `@vitest/*` dep): `vitest` removed from deps and all resolution mechanisms, no pin added.
  - npm/bun/yarn variants; user-authored non-wrapper `vitest`/coverage range preserved with a warning.
- **Snap tests** (`packages/cli/snap-tests-global/`): committed `migration-upgrade-v0_1-*` fixtures for both branches (direct-usage = urllib mirror, common-case = removal), per package manager, plus an idempotency fixture running `vp migrate` twice. Inputs must be committed files.
- **Routing test** (`crates/vite_global_cli`): local `vite-plus` older than global `vp` runs the global migrate path; equal stays local-first.
- **E2E**: real urllib, run the upgrade, assert no wrapper refs, single `vitest@4.1.9`, `@vitest/coverage-v8@4.1.9`, and `vp run cov` passes with no skew warning.

## Rollout

1. Land the empty-`pnpm` misrouting fix (Open Question 3) as a standalone bugfix with a regression test, independent of the rest.
2. Ship the full upgrade behavior, then update the v0.2.x release notes / `docs/guide/upgrade.md` to recommend `vp upgrade && vp migrate` and remove the "do not run `vp migrate`" disclaimer.
3. `npm deprecate @voidzero-dev/vite-plus-test "Merged into vite-plus; run 'vp upgrade && vp migrate' to upgrade your project"`.

## Alternatives Considered

- **Keep #1588's always-pin behavior** (write `vitest: VITEST_VERSION` for every project). Rejected: the prompt removes vitest in the common case precisely so future `vp update vite-plus` keeps vitest correct without a project pin to drift. Always-pinning creates per-release maintenance and redundant config.
- **Auto-heal in `vp install`**: rejected as primary mechanism (install should not rewrite config unprompted); a discovery warning pointing at `vp migrate` is a follow-up (Open Question 2). It must live in the global routing layer to reach stale-local-CLI projects.
- **Always delegate `migrate` to the global CLI** (drop local-first for this command). Simpler than the version check; changes behavior for users who pin a local version. Open Question 1.

## Open Questions

1. **Routing**: local-vs-global version check (recommended) vs. always routing `migrate` through the global CLI? Comparison rule: any-older or only cross-major?
2. Should `vp install` / `vp outdated` warn when a stale wrapper alias or behind `vite-plus` is present, pointing at `vp migrate`? To reach stale-local-CLI projects the warning must live in the global routing layer.
3. The empty-`pkg.pnpm` misrouting is a standalone #1588 bug. Ship it as a separate fix first, with a regression test for the `"pnpm": {}` + `pnpm-workspace.yaml` shape?
4. **Exact vs `latest`**: the prompt pins `vite-plus` and the core alias to the exact target; the current migrate convention writes `@latest` / `catalog: latest`. Should the upgrade path write exact versions (recommended, guarantees the lockfile moves and matches the prompt), and should normal migrate adopt the same?
5. **Removal default under `--no-interactive`**: removing `vitest` and resolution config is more invasive than pinning. Acceptable unattended in CI, or gated behind an explicit flag the first time?
6. Do we want `vp migrate --check` (detection only, exit code signals an available upgrade) for CI, mirroring `vp upgrade --check`?
7. **Direct-usage detection fidelity**: is "any `@vitest/*` listed, or any direct `vitest`/`@vitest` import" sufficient, or do we also need to catch indirect integration packages (`vitest-browser-*`, framework test plugins) that imply vitest usage without a direct import?

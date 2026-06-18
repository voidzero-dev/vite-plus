# RFC: `vp migrate` Upgrade Path for Existing Vite+ Projects

- Status: Draft (for discussion)
- Depends on: [#1588 refactor: replace @voidzero-dev/vite-plus-test with upstream vitest](https://github.com/voidzero-dev/vite-plus/pull/1588) (merged, `342fd2f4`)
- Related: [migration-command.md](./migration-command.md), [upgrade-command.md](./upgrade-command.md), `docs/guide/upgrade.md`

## Background

PR #1588 (shipped in v0.2.0) deleted the bundled `@voidzero-dev/vite-plus-test` wrapper and consumes upstream `vitest` directly. The managed dependency shape it writes is:

- `vite` stays aliased to `npm:@voidzero-dev/vite-plus-core@latest` (unchanged).
- `vitest` is pinned to the bundled `VITEST_VERSION` (currently `4.1.9`, in `packages/cli/src/utils/constants.ts`). The `@vitest/*` runtime family (`expect`, `runner`, `snapshot`, `spy`, `utils`, `mocker`, `pretty-format`) are EXACT dependencies of `vitest` itself, so a single `vitest` override cascades one consistent version to the whole tree. They are deliberately NOT pinned individually.
- The package-manager age gate gets `VITEST_AGE_GATE_EXEMPT_PACKAGES = ['vitest', '@vitest/*']` added (pnpm `minimumReleaseAgeExclude` / Yarn `npmPreapprovedPackages`) so the freshly published pinned version is not quarantined.
- Coverage providers (`@vitest/coverage-v8` / `@vitest/coverage-istanbul`) are NOT managed at all: they are peer deps the project installs and versions itself. A runtime guard in `packages/cli/src/define-config.ts` fail-fasts when an installed provider's version skews from the bundled vitest (Vitest otherwise silently runs mixed versions and yields unreliable coverage).

So the canonical v0.2.0 shape, pnpm monorepo (`pnpm-workspace.yaml`):

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: 4.1.9
  vite-plus: latest
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny: [vite, vitest]
  allowedVersions: { vite: '*', vitest: '*' }
minimumReleaseAgeExclude:
  - vite-plus
  - '@voidzero-dev/*'
  # ... oxlint/oxfmt families ...
  - vitest
  - '@vitest/*'
```

npm/bun standalone (`package.json`):

```jsonc
"overrides": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "4.1.9"
}
```

### What #1588 already handles

PR #1588 did not stop at the rewrite functions. It also added an "existing Vite+ project" repair path that this RFC originally proposed:

- `detectVitePlusBootstrapPending` (`migrator.ts`) inspects, per package manager, whether an already-Vite+ project's override shape is stale, including the case where `vitest` still points at the deleted `@voidzero-dev/vite-plus-test` wrapper (`isSemanticVitePlusOverrideSpec` treats a wrapper alias as NOT satisfied).
- `ensureVitePlusBootstrap` (`migrator.ts`) rewrites overrides/resolutions/catalog/peerDependencyRules to the canonical shape for npm, yarn, bun, and pnpm.
- `bin.ts` wires both into the "already using Vite+" early-return path and triggers one reinstall via `handleInstallResult`.

This is proven by the `migration-already-vite-plus` snap fixture, whose input has `"vitest": "npm:@voidzero-dev/vite-plus-test@latest"` in `overrides` and whose output rewrites it to the bundled `vitest` version, even under `--no-interactive`.

### The real gap: upgrading a v0.1.x project (urllib, 0.1.24 -> 0.2.0)

Running `vp migrate` in a real 0.1.24 project (`node-modules/urllib`) did NOT upgrade the vitest stack. Its `package.json`:

```jsonc
{
  "devDependencies": {
    "@vitest/coverage-v8": "^4.1.8",
    "vite": "npm:@voidzero-dev/vite-plus-core@^0.1.24",
    "vite-plus": "^0.1.24",
    "vitest": "npm:@voidzero-dev/vite-plus-test@^0.1.24",
  },
  "overrides": {},
  "pnpm": {},
  "packageManager": "pnpm@11.7.0",
}
```

Three independent root causes, each enough to break the upgrade:

1. **Routing: the stale local CLI runs (primary cause).** `vp migrate` is delegated **local-first** (`crates/vite_global_cli/src/commands/delegate.rs`). urllib has `vite-plus@0.1.24` installed in `node_modules`, so the global `vp v0.2.0` delegated to the **0.1.24** migrate CLI, which predates #1588 and has no bootstrap/upgrade logic at all. None of the repair above ever executed. This is the chicken-and-egg: the project that most needs the new upgrade code is exactly the project whose installed CLI is too old to contain it.

2. **The v0.1.x inline-devDependency-alias shape is not repaired.** v0.1.x migration wrote the aliases directly into `devDependencies` (`vite`/`vitest` aliased to `@voidzero-dev/vite-plus-*@^0.1.24`) with a pinned `vite-plus: ^0.1.24` and empty `overrides`/`pnpm`. Even the v0.2.0 `ensureVitePlusBootstrap` does not fully fix this:
   - `ensureVitePlusDependencySpecs` only re-pins `vite-plus` when its spec is `catalog:` or absent. A pinned `^0.1.24` is left untouched, so `vite-plus` resolves to the newest `0.1.x` and never reaches `0.2.0`.
   - The inline `vite`/`vitest` alias entries in `devDependencies` are never rewritten, so `vitest` keeps naming the dead `@voidzero-dev/vite-plus-test` wrapper.
   - Writing catalog/overrides on top of the surviving inline aliases produces a confusing half-migrated state rather than the canonical shape.

3. **Coverage providers are never aligned.** `@vitest/coverage-v8: ^4.1.8` is intentionally outside `VITE_PLUS_OVERRIDE_PACKAGES`. Bootstrap does not touch it and the lockfile keeps `4.1.8`, so it lags the bundled `vitest@4.1.9`. The only feedback is the runtime skew warning/guard in `define-config.ts`, which fires when the user later runs `vp test --coverage`. The migration itself does nothing to bring the provider to `4.1.9`.

The user-visible symptom is exactly what was reported: after `vp migrate`, `vite-plus` is still `0.1.x` and `@vitest/coverage-v8` is still `4.1.8`, not the expected `4.1.9`.

### Why the documented v0.2.0 upgrade flow still fails

The v0.2.0 release notes document the upgrade as "bump `vite-plus` first, then migrate":

```bash
vp update vite-plus --latest
vp migrate
```

Following this on urllib still does not upgrade cleanly. The post-run state (directly observed) explains why:

- urllib has a committed `pnpm-workspace.yaml` written by the old 0.1.x CLI that actively **pins** the stack to 0.1.x:

  ```yaml
  overrides:
    vite: 'npm:@voidzero-dev/vite-plus-core@^0.1.24' # forces core to 0.1.x
    vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24' # forces the deleted wrapper
  ```

  So the stale CLI does not merely no-op; it has written overrides that block any later upgrade. Installed result: `vite-plus 0.1.24`, `vitest = @voidzero-dev/vite-plus-test 0.1.24`, `@vitest/coverage-v8 4.1.8`.

- `vp update vite-plus --latest` deliberately does NOT re-resolve these aliases/overrides (documented in `docs/guide/upgrade.md`), so the `^0.1.24` pins survive the bump. The `vite` override is a **behind core alias** (`core@^0.1.24`), not the dead wrapper, so #1588's `pruneLegacyWrapperAliases` (which only matches the `@voidzero-dev/vite-plus-test` wrapper) would not even normalize it.

- The `vp migrate` step delegates local-first to whatever `vite-plus` is installed; if the update did not actually move the installed CLI past 0.1.x (the override pins fight the bump), migrate re-runs the 0.1.x CLI and rewrites the same old-shape `pnpm-workspace.yaml`.

- Even when the v0.2.0 CLI does run, urllib's `package.json` carries an empty `"pnpm": {}`. Both `detectVitePlusBootstrapPending` and `ensureVitePlusBootstrap` branch on `if (pkg.pnpm)`, and `{}` is truthy, so they inspect `pkg.pnpm.overrides` (empty) and take the `if (!pkg.pnpm)` -> false path that **skips the `pnpm-workspace.yaml` rewrite entirely**. The result is the worst case: a fresh `pnpm.overrides` block is written into `package.json` while the pinning `overrides` in `pnpm-workspace.yaml` are left intact, leaving two conflicting override sources. This is effectively a bug in the #1588 logic: an empty/partial `pkg.pnpm` should be treated as "no package.json pnpm config" so the `pnpm-workspace.yaml` path runs.

So three structural blockers compound: stale-CLI-written pins, `vp update` not reconciling them, and the empty-`pnpm` misrouting that prevents the workspace-yaml repair. Coverage skew remains on top of all three.

## Goals

1. `vp migrate` upgrades a v0.1.x Vite+ project (e.g. `0.1.24`) to the current major (`0.2.0`) end to end: `vite-plus`, the `vite`/`vitest` aliases, and the coverage providers all land on versions consistent with the executing CLI.
2. Fix the routing so the upgrade is performed by a CLI new enough to contain the upgrade logic, instead of silently delegating to the stale local `vite-plus`.
3. Repair the v0.1.x inline-devDependency-alias shape (aliases in `devDependencies`, pinned `vite-plus`, empty `overrides`) into the canonical v0.2.0 shape, in addition to the override-based shape #1588 already handles.
4. Align coverage providers (`@vitest/coverage-v8` / `@vitest/coverage-istanbul`) to the bundled `VITEST_VERSION` during migration, turning the runtime skew warning into a migration-time auto-fix.
5. Idempotent and conservative: a re-run on an upgraded project is a no-op, and user-authored, non-wrapper specs are preserved (same stance as #1588's prune sweeps).

## Non-Goals

- Changing behavior for projects that do not have `vite-plus` yet; the full-migration path already writes the canonical shape.
- A general project codemod for user source. `vite-plus/test*` remains the stable public API, so no source imports need to change.
- Replacing `vp update` / `vp outdated` for routine version bumps; `vp migrate` repairs shape and performs the cross-major upgrade, not day-to-day patching.
- Pinning the `@vitest/*` runtime family individually. They cascade from the single `vitest` pin and must stay that way (see Background). Coverage providers are the one exception this RFC adds, because they are independently installed peers, not transitive deps of `vitest`.

## Design

### 1. Routing: never let a stale local CLI silently own the upgrade

This is the crux. `vp migrate` must guarantee the migrate logic that runs is at least as new as the global `vp` performing the command.

Proposal: a cheap pre-delegation check in the global CLI (`crates/vite_global_cli`). Before `delegate_to_local_cli` for `migrate`:

- Read the local `vite-plus` version (from `node_modules/vite-plus/package.json`, already resolvable on the delegation path).
- Compare it to the global `vp` version.
- If the local `vite-plus` is older than the global `vp` (cross-major or otherwise behind), do NOT delegate to the local CLI. Run `migrate` from the **global** JS CLI instead (`delegate_to_global_cli`). The global CLI is the same `vite-plus` package at the global version, so `VITE_PLUS_VERSION` / `VITEST_VERSION` / the writers are all consistent and the upgrade targets `0.2.0`.

This keeps local-first semantics for the normal case (local == global, or local newer) and only escalates when the local CLI is provably too old to perform the upgrade. The global CLI then re-pins `vite-plus` to its own version, so the very next `vp` invocation in the project picks up the upgraded local CLI.

This is not optional polish: the urllib evidence shows the stale local CLI does not just fail to upgrade, it writes `pnpm-workspace.yaml` overrides that pin `vite`/`vitest` to `^0.1.24` and the deleted wrapper, actively blocking later upgrades. The documented "bump first" flow (`vp update vite-plus --latest && vp migrate`) does not reliably escape this, because `vp update` does not reconcile those pins (by design) and the bump can be fought by the pins themselves. Routing the upgrade to a CLI that is new enough to repair the shape is the only robust fix.

Alternative (simpler, listed in Open Questions): always route `migrate` through the global CLI, dropping local-first for this one command, on the grounds that migrate is a toolchain-level operation like `create`.

### 2. Detect the v0.1.x shape (state-based)

Extend the existing detection so `detectVitePlusBootstrapPending` (or a sibling) also flags:

- A `vite-plus` dependency spec that is a concrete range/version **older than the executing CLI's major/version** (e.g. `^0.1.24` when the CLI is `0.2.0`), not just `catalog:`/absent.
- `vite`/`vitest` **inline alias** entries in any dependency field pointing at `npm:@voidzero-dev/vite-plus-core@*` or the `@voidzero-dev/vite-plus-test` wrapper, regardless of whether an `overrides`/`catalog` block exists.
- Installed coverage providers whose version does not satisfy the bundled `VITEST_VERSION`.

Detection stays cheap (file reads, plus the already-available installed-version info), no network.

### 3. Repair the v0.1.x shape

Extend `ensureVitePlusBootstrap` (or the upgrade fixup it calls) so that, in addition to the override/catalog reconciliation it already does:

1. **Re-pin `vite-plus`** to the executing CLI's target spec whenever the current spec resolves below the CLI version, not only when it is `catalog:`. For pnpm/bun monorepos this becomes `catalog:` with a `vite-plus: latest` (or the CLI version) catalog entry; for standalone it becomes the explicit version. This is what moves `0.1.24 -> 0.2.0`.
2. **Normalize inline and behind aliases.** Reuse the #1588 prune helpers (`pruneLegacyWrapperAliases`) at the dependency-field level for the dead `vitest: npm:@voidzero-dev/vite-plus-test@*` wrapper, and ADD normalization for **behind core aliases** that the prune does not catch: any `vite: npm:@voidzero-dev/vite-plus-core@<old>` (e.g. `@^0.1.24`) is realigned to `@latest`. Apply this in both `package.json` dependency fields and the override/catalog/`pnpm-workspace.yaml` blocks, then move management into the canonical block so surviving entries match the v0.2.0 shape rather than carrying stale pins.
3. **Reconcile the managed block** to the canonical shape (the part #1588 already does): `overrides`/`resolutions`/`catalog` for `vite` and `vitest`, pnpm `peerDependencyRules`, and the `vitest` / `@vitest/*` age-gate exemptions. User-authored, non-wrapper specs are left alone and reported as a warning instead of being overwritten.
4. **Repair the right pnpm location.** Treat an empty/partial `pkg.pnpm` (e.g. `"pnpm": {}`) as "no package.json pnpm config" so the `pnpm-workspace.yaml` path runs (fixes the misrouting in section Background). When both a `package.json` `pnpm.overrides` and a `pnpm-workspace.yaml` `overrides` exist, reconcile both so the project is not left with two conflicting override sources; prune the stale `^0.1.24`/wrapper pins from whichever location holds them.

### 4. Align coverage providers

When migrating, detect `@vitest/coverage-v8` / `@vitest/coverage-istanbul` in any dependency field and rewrite their spec to the bundled `VITEST_VERSION` (e.g. `^4.1.8` -> `4.1.9`), so the installed provider matches the runner and the `define-config.ts` guard stays quiet. This is the migration-time counterpart to that runtime guard: the guard remains the safety net for projects that never re-run migrate, while migrate proactively fixes the version it already knows the correct value for.

- Only rewrite providers that are already present; never add a coverage provider the project did not have.
- Reuse the same name resolution the runtime guard uses (`@vitest/coverage-<provider>`) so the set stays in sync.
- Report each aligned provider in the migration summary.

### 5. Install and verification

If any repair mutated files, run a single `vp install` with `--no-frozen-lockfile` (pnpm/yarn) or `--force` (npm/bun), reusing `handleInstallResult` so failures surface as warnings and a non-zero exit code (mirrors #1588). After install, verify:

- Zero `@voidzero-dev/vite-plus-test` references remain in the lockfile.
- The resolved `vite-plus`, `vitest`, and any coverage provider are at the expected versions.

Emit a warning listing any offending keys if a check fails (e.g. a transitive dep the prune could not reach).

### 6. UX

Interactive:

```
$ vp migrate
│ This project uses an older Vite+ (0.1.24); the global CLI is 0.2.0.
│ Detected setup written by an older Vite+ version:
│   - vite-plus is pinned to 0.1.x
│   - vitest is aliased to the removed @voidzero-dev/vite-plus-test wrapper
│   - @vitest/coverage-v8 (4.1.8) does not match the bundled vitest (4.1.9)
◆ Upgrade this project to Vite+ 0.2.0?
│   Re-pins vite-plus, rewrites the vitest setup to upstream vitest 4.1.9,
│   aligns coverage providers, and reinstalls.
│ ● Yes / ○ No
```

- One prompt for the whole upgrade, not one per change.
- `--no-interactive` applies the upgrade (declining would leave the project broken-by-default; matches migrate's convention of applying safe defaults). See Open Question 5 for whether an unattended cross-major bump in CI should require an explicit flag.
- Summary section, fed by `MigrationReport`:

```
Upgraded Vite+ 0.1.24 -> 0.2.0
  re-pinned vite-plus
  rewrote stale vitest wrapper alias -> vitest 4.1.9
  aligned @vitest/coverage-v8 4.1.8 -> 4.1.9
```

### 7. Idempotency

After a successful upgrade, detection returns false (no wrapper aliases, `vite-plus` at target, providers aligned), so a re-run takes the existing "already using Vite+, happy coding" path. Repairs must be recoverable by re-running `vp migrate` if an install fails after files were rewritten.

## Code Touchpoints

| Area                                                                                       | Change                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/vite_global_cli/src/commands/migrate.rs` (+ `delegate.rs`, version compare helper) | Pre-delegation local-vs-global version check; route `migrate` to the global CLI when the local `vite-plus` is older                                                                                                                                                                                                           |
| `packages/cli/src/migration/migrator.ts`                                                   | Extend `detectVitePlusBootstrapPending` and `ensureVitePlusBootstrap`: re-pin a behind `vite-plus` (not only `catalog:`), normalize inline/behind `vite`/`vitest` aliases, align coverage providers, treat empty `pkg.pnpm` as "no pnpm config" so the `pnpm-workspace.yaml` path runs, and reconcile both override locations |
| `packages/cli/src/migration/bin.ts`                                                        | Surface the version bump and coverage alignment in the existing already-Vite+ path and summary                                                                                                                                                                                                                                |
| `packages/cli/src/migration/report.ts`                                                     | Report fields for version bump and coverage alignment                                                                                                                                                                                                                                                                         |
| `docs/guide/upgrade.md`                                                                    | Section: upgrading a v0.1.x project (`vp upgrade && vp migrate`), note that `vp migrate` re-pins and reinstalls                                                                                                                                                                                                               |
| `docs/guide/migrate.md`                                                                    | Note that migrate on an existing Vite+ project performs the cross-version upgrade                                                                                                                                                                                                                                             |

## Testing Plan

- **Unit** (`migrator.spec.ts`): the urllib shape (pnpm, inline `vite`/`vitest` aliases in `devDependencies`, pinned `vite-plus: ^0.1.24`, empty `overrides`/`pnpm`, `@vitest/coverage-v8: ^4.1.8`) detected as pending and repaired to: `vite-plus` at target, no wrapper alias, coverage provider at `VITEST_VERSION`. Plus the npm/bun/yarn equivalents and a user-authored non-wrapper `vitest`/coverage range preserved with a warning.
- **Snap tests** (`packages/cli/snap-tests-global/`): a committed `migration-upgrade-v0_1-inline-alias-pnpm` fixture that mirrors urllib EXACTLY (inline `vite`/`vitest` aliases in `devDependencies`, pinned `vite-plus: ^0.1.24`, empty `"pnpm": {}`, AND a committed `pnpm-workspace.yaml` whose `overrides` pin `vite`/`vitest` to `@^0.1.24`/the wrapper, plus `@vitest/coverage-v8: ^4.1.8`). Assert the output has no `^0.1.24`/wrapper pins in either location, a single override source, and aligned coverage. Add standalone npm/yarn/bun variants and an idempotency fixture running `vp migrate` twice. Inputs must be committed files.
- **Routing test** (`crates/vite_global_cli`): with a local `vite-plus` older than the global `vp`, `vp migrate` runs the global migrate path; with local == global it stays local-first.
- **E2E**: a real 0.1.24-shaped project (urllib), run `vp migrate`, assert `vite-plus`, `vitest`, and `@vitest/coverage-v8` resolve to the expected versions and `vp test --coverage` passes with no skew warning.

## Rollout and Complementary Actions

1. `npm deprecate @voidzero-dev/vite-plus-test "Merged into vite-plus; run 'vp upgrade && vp migrate' to upgrade your project"` so users who never re-run migrate get a pointer at install time.
2. Release notes and `docs/guide/upgrade.md` document the one-command upgrade: `vp upgrade && vp migrate`.

## Alternatives Considered

- **Auto-heal in `vp install`**: detect and fix on every install. Rejected as primary mechanism (install should not rewrite config files unprompted), but a lightweight **warning** in `vp install` pointing at `vp migrate` is proposed as a follow-up (Open Question 2). Note this would also have the stale-local-CLI problem unless the warning lives in the global routing layer.
- **Hook into `vp update vite-plus`**: reconcile shape on every bump. Splits migration logic across commands and misses hand edits. Per `docs/guide/upgrade.md`, `vp update` deliberately does not re-resolve the aliases, so this would be a behavior change.
- **Version-marker file** recording the migrating Vite+ version. Rejected: state-based detection is robust to hand-edits and needs no new artifact in user repos.
- **Always delegate `migrate` to the global CLI** (drop local-first for this command). Simpler than the version check; arguably correct since migrate is toolchain-level. Changes behavior for users who intentionally pin a local version. Kept as Open Question 1.

## Open Questions

1. **Routing**: pre-delegation local-vs-global version check (recommended) vs. always routing `migrate` through the global CLI? The check preserves local-first for the normal case; always-global is simpler but a behavior change. Either way, what is the comparison rule (any-older, or only cross-major)?
2. Should `vp install` / `vp outdated` **warn** when a stale wrapper alias or a behind `vite-plus` is present, pointing at `vp migrate`? Main discovery path for users who do not re-run migrate. To work for stale-local-CLI projects, the warning must live in the global routing layer.
3. When the project has a **user-authored, non-wrapper `vitest` range** (someone opted out of the managed pin), should migrate still re-pin to `VITEST_VERSION` and align coverage, or skip with a warning? Current proposal: preserve the user's `vitest`, warn, and skip coverage alignment for that project to avoid forcing a mixed tree.
4. Coverage alignment policy: always rewrite to the exact bundled `VITEST_VERSION`, or to a compatible caret range? Exact matches the runtime guard's expectation (the guard wants an exact-version match); a caret could drift again. Current proposal: exact.
5. Should a cross-major bump under `--no-interactive` (CI) be automatic, or require an explicit `--upgrade` flag the first time, so CI does not get an unattended major bump?
6. Do we want `vp migrate --check` (detection only, exit code signals an available upgrade) for CI, mirroring `vp upgrade --check`?
7. The empty-`pkg.pnpm` misrouting (Background) is arguably a standalone bug in #1588 worth fixing immediately, independent of the rest of this RFC. Should it ship as a separate fix first, with a regression test for the `"pnpm": {}` + `pnpm-workspace.yaml` shape?
8. Should the v0.2.0 release notes upgrade flow be corrected? As written (`vp update vite-plus --latest && vp migrate`) it does not reliably upgrade projects with stale pinning overrides; the recommended flow may need to be `vp upgrade` (global) then `vp migrate`, once routing escalates to the global CLI.

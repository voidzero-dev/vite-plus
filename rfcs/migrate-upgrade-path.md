# RFC: `vp migrate` Upgrade Path for Existing Vite+ Projects

- Status: Draft (for discussion)
- Depends on: [#1588 refactor: replace @voidzero-dev/vite-plus-test with upstream vitest](https://github.com/voidzero-dev/vite-plus/pull/1588)
- Related: [migration-command.md](./migration-command.md), [upgrade-command.md](./upgrade-command.md), `docs/guide/upgrade.md`

## Background

PR #1588 deletes the bundled `@voidzero-dev/vite-plus-test` wrapper and consumes upstream `vitest` directly. New migrations write a different dependency shape: `vitest` and nine `@vitest/*` internals are pinned to the bundled `VITEST_VERSION` in the package-manager override mechanism, instead of aliasing `vitest` to the wrapper.

Every project migrated **before** #1588 carries the old shape on disk. Per package manager:

| Package manager               | Location                                               | Stale entry                                                   |
| ----------------------------- | ------------------------------------------------------ | ------------------------------------------------------------- |
| pnpm                          | `pnpm-workspace.yaml` `catalog` (and named `catalogs`) | `vitest: npm:@voidzero-dev/vite-plus-test@latest` (or pinned) |
| pnpm (existing `pnpm` config) | `package.json` `pnpm.overrides`                        | same alias                                                    |
| npm                           | `package.json` `overrides`                             | `"vitest": "npm:@voidzero-dev/vite-plus-test@latest"`         |
| bun                           | `package.json` `overrides` / `workspaces.catalog`      | same alias                                                    |
| yarn                          | `package.json` `resolutions`                           | same alias                                                    |
| all                           | dependency fields (`devDependencies` etc.)             | `vitest` aliased to the wrapper in some setups                |
| all                           | lockfile                                               | resolved `@voidzero-dev/vite-plus-test` entries               |

Old projects also **lack** entries the new shape requires:

- The nine `@vitest/*` override/catalog pins (`@vitest/expect`, `runner`, `snapshot`, `spy`, `utils`, `mocker`, `pretty-format`, `coverage-v8`, `coverage-istanbul`).
- The expanded pnpm `peerDependencyRules` (`allowAny` / `allowedVersions` for those packages).
- The pnpm `allowBuilds` entries for browser-provider drivers.

### What breaks if we do nothing

The wrapper stays published on npm (existing versions are immutable), so installs do not hard-fail. The failure modes are quieter and worse:

1. **Permanently stale vitest.** The wrapper receives no further releases. The override forces every `vitest` in the tree, including the one `vite-plus` itself depends on, to the last wrapper version. Users never receive vitest updates or security fixes again, regardless of how often they update `vite-plus`.
2. **Mixed vitest copies.** The unpinned `@vitest/*` internals resolve to the newest 4.x while `vitest` is pinned to the old wrapper. Two physical vitest module graphs is the classic source of mock-hoisting and internal-state bugs.
3. **Peer conflicts.** New `vite-plus` ships `@vitest/browser-*` providers with exact `vitest` peers. The override forces a non-matching version into the tree.
4. **Dead-end update advice.** `docs/guide/upgrade.md` previously told users to run `vp update @voidzero-dev/vite-plus-test`, which now updates to a package that will never move.

#1588 already adds `pruneLegacyWrapperAliases` / `pruneYamlMapLegacyWrapperAliases` sweeps in `migrator.ts`, but they only run inside the **full migration** path (`rewritePackageJson` / `rewriteConfigs`). When a project already has `vite-plus` as a dependency, `bin.ts` takes the early-return path (`packages/cli/src/migration/bin.ts`, "Early return if already using Vite+") which only offers ESLint, Prettier, git hooks, baseUrl, and node-version migrations. The stale override shape is never touched. That gap is what this RFC closes.

## Goals

1. `vp migrate` on a project that already uses Vite+ detects state written by older Vite+ versions and repairs it automatically.
2. The first (and motivating) repair: replace stale `@voidzero-dev/vite-plus-test` aliases with the upstream-vitest shape and add the missing `@vitest/*` pins, across all four package managers, in standalone projects and monorepos.
3. Establish a small, ordered **upgrade-fixups registry** so future breaking changes in the managed dependency shape get the same treatment without redesigning the flow.
4. Idempotent: re-running `vp migrate` on an already-repaired project is a no-op.
5. Conservative: user-authored specs that are not wrapper aliases are preserved (same stance as #1588's prune sweeps).

## Non-Goals

- Changing the behavior of `vp migrate` for projects that do not have `vite-plus` yet (the full-migration path already handles stale aliases after #1588).
- A general project codemod system for arbitrary user code. Import rewrites from the original migration are not re-run; `vite-plus/test*` remains the stable public API, so no source files need to change.
- Replacing `vp update` / `vp outdated`. Those remain the way to bump versions day to day; `vp migrate` repairs **shape**, not routine version bumps.

## Design

### 1. Detection: state-based, not version-based

There is no reliable marker recording which Vite+ version performed the original migration, and config files may have been hand-edited since. Detection therefore inspects the actual state:

- Scan `package.json` (`overrides`, `resolutions`, `pnpm.overrides`, `workspaces.catalog(s)`, dependency fields) and `pnpm-workspace.yaml` (`catalog`, `catalogs`, `overrides`) for specs matching `npm:@voidzero-dev/vite-plus-test` or `npm:@voidzero-dev/vite-plus-test@*` (reuse `isLegacyWrapperSpec` from #1588).
- Independently, detect a **vp-managed override block** (identified by the `vite: npm:@voidzero-dev/vite-plus-core@...` alias) that is missing keys from the current `VITE_PLUS_OVERRIDE_PACKAGES`. This catches projects where someone hand-removed the wrapper alias but still lacks the `@vitest/*` pins.

Either signal marks the project as needing the fixup. Detection is cheap (file reads, no network, no install).

### 2. Upgrade-fixups registry

A new module `packages/cli/src/migration/upgrade-fixups.ts`:

```ts
interface UpgradeFixup {
  /** Stable id, e.g. 'vitest-wrapper-removal' */
  id: string;
  /** One-line description shown in the prompt and the summary */
  summary: string;
  /** Cheap, read-only check against the workspace */
  detect(workspace: WorkspaceInfo): boolean;
  /** Mutates config files; returns what changed for the report */
  apply(workspace: WorkspaceInfo, report: MigrationReport): Promise<FixupResult>;
}

export const UPGRADE_FIXUPS: UpgradeFixup[] = [vitestWrapperRemovalFixup];
```

The already-using-Vite+ path in `bin.ts` runs `detect()` for each registered fixup, prompts once for the batch (see UX below), applies them in order, and triggers a single reinstall if any fixup mutated files. Future breaking changes (for example, if the `vite` alias shape ever changes) append a new entry instead of growing ad-hoc branches.

### 3. Fixup #1: vitest wrapper removal

`apply()` reuses the #1588 machinery rather than introducing new rewrite logic:

1. **Prune wrapper aliases** everywhere they can appear, via `pruneLegacyWrapperAliases` (JSON records: `overrides`, `resolutions`, `pnpm.overrides`, dependency fields, bun `workspaces.catalog(s)`) and `pruneYamlMapLegacyWrapperAliases` (pnpm-workspace.yaml `catalog`, named `catalogs`, `overrides`). `vitest` keys are rewritten to `VITEST_VERSION` so existing `catalog:` references keep resolving; other wrapper-targeted keys are dropped.
2. **Reconcile the managed block to the canonical shape**: for the override mechanism the project already uses, ensure every key in `VITE_PLUS_OVERRIDE_PACKAGES` is present with the canonical value, and extend pnpm `peerDependencyRules` / `allowBuilds` the same way the full migration writes them (reuse the existing per-package-manager writers in `migrator.ts`). Existing keys whose value is a user-authored, non-wrapper spec are left alone and reported as a warning instead of being overwritten.
3. **Walk workspace packages** in monorepos: each package's `package.json` dependency fields get the same prune (mirrors the #1588 sweep at the dependency-field level).

Before/after, pnpm monorepo (`pnpm-workspace.yaml`):

```yaml
# before (written by older vp migrate)
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
  vite-plus: latest
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny: [vite, vitest]
  allowedVersions: { vite: '*', vitest: '*' }

# after
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: 4.1.7
  '@vitest/expect': 4.1.7
  # ... runner, snapshot, spy, utils, mocker, pretty-format, coverage-v8, coverage-istanbul
  vite-plus: latest
allowBuilds:
  edgedriver: false
  geckodriver: false
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
  '@vitest/expect': 'catalog:'
  # ... same set
peerDependencyRules:
  allowAny: [vite, vitest, '@vitest/expect', ...]
  allowedVersions: { vite: '*', vitest: '*', '@vitest/expect': '*', ... }
```

Before/after, npm/bun standalone (`package.json`):

```jsonc
// before
"overrides": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
}

// after
"overrides": {
  "vite": "npm:@voidzero-dev/vite-plus-core@latest",
  "vitest": "4.1.7",
  "@vitest/expect": "4.1.7"
  // ... same set
}
```

yarn `resolutions` follows the npm shape.

### 4. Bumping `vite-plus` itself

The repaired shape pins `vitest` to the `VITEST_VERSION` baked into the CLI performing the migration. That pin is only correct if the project's `vite-plus` version bundles the same vitest. The fixup therefore also normalizes the `vite-plus` spec:

- If the spec is a dist-tag (`latest`) or a range satisfied by the CLI's own version, leave it; the reinstall resolves it forward.
- Otherwise (older pinned version), update it to the migrating CLI's version, the same way the full migration pins `vite-plus` today (`catalog:` in monorepos, explicit version standalone).

This keeps `vite-plus` and the vitest pins in lockstep by construction, because the executing CLI writes both from its own constants.

### 5. Install and verification

If any fixup mutated files, run a single `vp install` with `--no-frozen-lockfile` (pnpm/yarn) or `--force` (npm/bun), reusing `handleInstallResult` from #1588 so failures surface as warnings and a non-zero exit code. After install, verify the lockfile contains zero `@voidzero-dev/vite-plus-test` references; if any remain (for example, a transitive dependency the prune could not reach), emit a warning with the offending lockfile keys.

### 6. Command routing: the chicken-and-egg problem

`vp migrate` is delegated **local-first** (`crates/vite_global_cli/src/commands/delegate.rs`): if the project has a local `vite-plus`, its (old) migration code runs, which knows nothing about the new shape. Meanwhile the user cannot cleanly get the new local `vite-plus` first, because installing it under the stale overrides produces the mixed-vitest state described above.

The global `vp` binary is the natural escape hatch: users keep it current via `vp upgrade`, independent of any project. Proposal:

**Global preflight (recommended).** Before delegating `migrate`, the global CLI runs the cheap stale-state scan itself (or always routes `migrate` for already-Vite+ projects through the global JS CLI). When stale wrapper state is detected, the **global** CLI's migration code executes the fixups, then proceeds with the existing partial migrations. Since the global JS CLI is the same `vite-plus` package at the global version, `VITEST_VERSION` and the writers are automatically consistent.

Alternative routings are listed under Open Questions.

### 7. UX

Interactive:

```
$ vp migrate
│ This project already uses Vite+.
│ Detected configuration written by an older Vite+ version:
│   - vitest is aliased to the removed @voidzero-dev/vite-plus-test wrapper
◆ Upgrade the Vite+ dependency setup?
│   Rewrites catalog/overrides to upstream vitest 4.1.7, updates vite-plus, reinstalls.
│ ● Yes / ○ No
```

- One prompt for the whole fixup batch, not one per fixup; the bullet list names each detected fixup via its `summary`.
- `--no-interactive` applies the fixups (declining would leave the project broken-by-default; this matches migrate's existing convention of applying safe defaults). Declining interactively prints the manual steps and continues with the other partial migrations.
- The migration summary gains a section, fed by `MigrationReport`:

```
Upgraded Vite+ dependency setup
  rewrote 2 stale vitest aliases (pnpm-workspace.yaml, packages/app/package.json)
  added 9 @vitest/* pins
  vite-plus: 0.6.0 -> 0.9.0
```

### 8. Idempotency

After a successful run, `detect()` returns false for every fixup (no wrapper aliases, no missing keys), so a re-run takes the existing "already using Vite+, happy coding" path. Fixups must be written so that partial failure (e.g. install failed after files were rewritten) is recoverable by simply re-running `vp migrate`.

## Code Touchpoints

| Area                                                               | Change                                                                                                                                                                     |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/cli/src/migration/upgrade-fixups.ts` (new)               | Fixup interface, registry, vitest-wrapper fixup                                                                                                                            |
| `packages/cli/src/migration/bin.ts`                                | Already-Vite+ path: run detection, prompt, apply, fold into the existing single-reinstall logic                                                                            |
| `packages/cli/src/migration/migrator.ts`                           | Export/reshape `pruneLegacyWrapperAliases`, `pruneYamlMapLegacyWrapperAliases`, and the per-PM override writers so the fixup can call them outside the full-migration flow |
| `packages/cli/src/migration/report.ts`                             | `upgradeFixups` entries (id, counts, version bump)                                                                                                                         |
| `crates/vite_global_cli/src/commands/migrate.rs` (+ `delegate.rs`) | Routing change per the preflight decision                                                                                                                                  |
| `docs/guide/upgrade.md`                                            | New section: upgrading projects migrated by older Vite+ (`vp migrate` repairs the setup)                                                                                   |
| `docs/guide/migrate.md`                                            | Note that running migrate on an existing Vite+ project also repairs older setups                                                                                           |

## Testing Plan

- **Unit** (`packages/cli/src/migration/__tests__/upgrade-fixups.spec.ts`): detection and apply for each stale shape: pnpm catalog + named catalogs, `pnpm.overrides` in package.json, npm/bun `overrides`, bun `workspaces.catalog`, yarn `resolutions`, wrapper aliases in dependency fields, pinned wrapper versions (`npm:@voidzero-dev/vite-plus-test@4.0.5`), hand-edited blocks missing only `@vitest/*` keys, user-authored `vitest: ^4.0.0` ranges preserved with warning.
- **Snap tests** (`packages/cli/snap-tests-global/`): new fixtures whose inputs are committed old-shape projects, e.g. `migration-upgrade-stale-vitest-pnpm`, `-npm`, `-yarn`, `-bun`, `migration-upgrade-monorepo-catalog`, plus an idempotency fixture that runs `vp migrate` twice and snapshots the second run's "already using Vite+" output. Fixture inputs must be committed files, not generated by ignored local state.
- **E2E**: take a project migrated by the last pre-#1588 release, run new `vp migrate`, assert `vp test` passes and the lockfile has zero wrapper references.

## Rollout and Complementary Actions

1. Land after #1588 merges and ships in the same release if possible, so the first release without the wrapper is also the first that can repair old projects.
2. `npm deprecate @voidzero-dev/vite-plus-test "Merged into vite-plus; run 'vp migrate' to update your project"` so users who never run migrate still get a pointer at install time.
3. Release notes and `docs/guide/upgrade.md` call out the one-command repair: `vp upgrade && vp migrate`.

## Alternatives Considered

- **Auto-heal in `vp install`**: detect stale aliases on every install and fix silently. Rejected as the primary mechanism: install should not rewrite config files unprompted, and the migration machinery (prompts, report, per-PM writers) already lives in migrate. A lightweight **warning** in `vp install` pointing at `vp migrate` is proposed as a follow-up (Open Question 2).
- **Hook into `vp update vite-plus`**: reconcile overrides whenever the vite-plus spec is bumped. More magical, splits migration logic across commands, and misses users who edit package.json by hand. The install-time warning covers discovery instead.
- **Version-marker file** (e.g. recording the migrating Vite+ version) to drive upgrade steps by version range. Rejected: state-based detection is robust to hand-edits and requires no new artifact in user repos.
- **Always delegate `migrate` to the global CLI** (drop local-first for this command). Simpler routing than a preflight, and arguably correct since migrate is a toolchain-level operation like `create`, but it changes behavior for users who intentionally pin a local version. Kept as an option in Open Question 1.

## Open Questions

1. **Routing**: global preflight scan (recommended) vs. always routing `migrate` through the global CLI for already-Vite+ projects? The preflight keeps local-first semantics for everything else but adds a Rust-side (or pre-delegation JS) scan; always-global is simpler but a behavior change.
2. Should `vp install` (and/or `vp doctor`-style checks, `vp outdated`) **warn** when stale wrapper aliases are present, pointing at `vp migrate`? This is the main discovery mechanism for users who do not think to run migrate again.
3. When the fixup finds a **user-authored `vitest` range** (not a wrapper alias) inside an otherwise vp-managed override block, should we still add the `@vitest/*` pins (risking a mixed tree against their chosen vitest) or skip the whole block with a warning? Current proposal: add nothing, warn, and explain the risk.
4. Should declining the fixup interactively be allowed to proceed with the other partial migrations (current proposal), or should migrate stop early since the project is in a known-broken state?
5. Is bumping the `vite-plus` spec to the migrating CLI's version acceptable in non-interactive mode, or should non-interactive runs require an explicit `--upgrade` flag the first time? (CI running `vp migrate --no-interactive` would otherwise get an unattended dependency bump.)
6. Do we want `vp migrate --check` (detection only, exit code signals drift) for CI, mirroring `vp upgrade --check`?

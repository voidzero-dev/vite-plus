# RFC: Native declaration generation for `vp pack`

Status: Proposed. Researched on 2026-11-11 against [PR #3352](https://github.com/voidzero-dev/vite-plus/pull/3352), head `8e76399da0512e2b5fcc6bb0774d4e9cbc967015`.

## Decision

Use the `tsrs` dependency introduced by #3352 to generate declarations for projects that do not install `typescript`. Select `dts.generator: 'tsrs'` through a Vite+ adapter. Keep the project's existing compiler when it supplies TypeScript 5, 6, or 7, and honor an explicit generator. Preserve Oxc generation for isolated declarations.

The latest declaration plugin supports this integration. Its automatic selection does not select `tsrs`, even when that package is installed. Upgrading dependencies alone therefore does not provide the proposed default. The `tsrs` generator uses the TypeScript 7.1-compatible native API; `generator: 'tsgo'` remains a separate choice that loads the Microsoft `typescript` package. [Generator implementation](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.1/src/options.ts), [native API loading](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.1/src/tsgo.ts).

This makes a project-level `typescript` dependency unnecessary for supported `vp check` and `vp pack` workflows. It does not replace `tsc` tasks, the editor's compiler, Vue/Volar integrations, or dependencies that use the classic TypeScript API. Application type packages such as `@types/node` remain project dependencies when needed. Existing projects keep `typescript` unless a separate dependency audit establishes that they can remove it.

Ship the native default first in the experimental release built on #3352. Promote it after the declaration, packaging, and platform gates below pass. TypeScript 7.1 is still a beta, and upstream marks the `tsrs` generator as highly experimental.

## Version baseline

These are researched versions, not floating release requirements. Refresh the matrix before implementation and pin the versions used for release qualification.

| Component | PR #3352 baseline | Proposed target |
| --- | --- | --- |
| `tsdown`, `@tsdown/css`, `@tsdown/exe` | `0.23.4` | `0.23.7`, kept in lockstep |
| Declaration plugin bundled through tsdown | `rolldown-plugin-dts@0.28.8` | `0.30.1` |
| Core's direct declaration-plugin development dependency | Catalog `^0.28.0` | Align with the bundled `0.30.1` |
| Native compiler supplied by Vite+ | `tsrs@0.24.0-ts7.1.0-beta`, used by lint/check | Same exact package, also used by pack |
| Microsoft compiler used as a parity reference | Project/repository compiler varies | Exact `typescript@7.1.0-beta` matching tsrs's source pin |
| Bundled Rolldown | `1.2.18` | A validated `1.3.x` integration; tsdown `0.23.7` requires `~1.3.0` |

The npm registry currently labels TypeScript `7.0.2` as `latest`, `7.1.0-beta` as `beta`, and `7.1.0-dev.20261111.1` as `next`. The design targets the beta API already supported by the pinned tsrs release, not a presumed stable TypeScript 7.1 release. [TypeScript registry metadata](https://registry.npmjs.org/typescript), [tsrs release and upstream pin](https://github.com/oxc-project/tsrs/blob/v0.24.0/docs/releases/0.24.0.md).

Two integration constraints precede the dependency update:

- Core redirects tsdown's Rolldown imports to its shared bundle. Updating tsdown does not install an independent runtime Rolldown for pack. Qualify a Vite version with the selected Rolldown `1.3.x` before changing that shared bundle. If the combination is not ready, defer the tsdown upgrade. Do not force the current bundle past its supported range. [Core import rewriting](../packages/core/build-support/rewrite-imports.ts), [Rolldown 1.3 release warning](https://github.com/rolldown/rolldown/releases/tag/v1.3.0).
- Both target packages require Node `^22.18.0 || ^24.11.0 || >=26.0.0`. The PR's local CLI already declares this range; core still advertises Node 20 and a broader `>=24.11.0` range. Align core's metadata and document the change for direct core consumers. `vp migrate` can report incompatible runtime pins, but cannot make these dependencies run on Node 20. [tsdown manifest](https://github.com/rolldown/tsdown/blob/v0.23.7/package.json), [plugin manifest](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.1/package.json).

## User behavior

A plain TypeScript library with a supported tsconfig needs only its normal `vite-plus` installation:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/index.ts'],
    dts: true,
  },
});
```

If the project does not provide `typescript`, `vp pack` selects bundled tsrs. If it already provides TypeScript 5/6, the same configuration keeps the classic compiler. An existing project can select bundled tsrs without removing its other compiler:

```ts
pack: {
  dts: { generator: 'tsrs' },
}
```

Keep declaration enablement separate from compiler selection. Preserve `dts: false`, CLI overrides, executable defaults, and tsdown's inference from `types`, `typings`, `exports.types`, and `compilerOptions.declaration`. Installing tsrs must not start generating declarations for projects that previously disabled them. Apply the adapter after tsdown has resolved this enablement, including array/workspace configurations and watch reloads. [tsdown configuration resolution](https://github.com/rolldown/tsdown/blob/v0.23.7/src/config/options.ts).

Keep `dts.typecheck` at its upstream default, `false`. Declaration emit can still fail on emit diagnostics; a complete type check remains an explicit choice. Keep #3352's existing `lint.options.typeCheck` behavior. This upgrade does not turn on additional checking for all pack builds.

### Generator selection

Apply these rules in order for each enabled declaration build. Resolve project packages from the package that owns the build, with workspace inheritance bounded by the workspace root.

| Condition | Behavior |
| --- | --- |
| Explicit generator string or custom `Generator` object | Preserve it and its options. Validate requirements; do not replace it after an error. |
| Implicit generator with Vue/Volar, `resolver: 'tsc'`, or a requested classic-only capability | Use a compatible project TypeScript 5/6 compiler. If unavailable, report the specific requirement and migration action. |
| `isolatedDeclarations: true`, without conflicting requirements | Keep Oxc. `typecheck: true` needs a checking generator; report the conflict instead of ignoring it. |
| Project TypeScript 5/6 | Keep `tsc` and the selected project version. |
| Project TypeScript exposes the required declaration API | Use `tsgo` with that package's module URL. |
| Project TypeScript 7.0 | Retain the deprecated CLI backend that plugin `0.30.1` still supplies. Report its migration path; do not assume later plugin minors retain it. |
| No project TypeScript, supported tsconfig and native options | Use bundled `tsrs`, with its module URL supplied by core. |
| Unsupported or broken compiler installation/configuration | Fail before emission with a focused diagnostic. Do not reinterpret a broken installation as an absent compiler. |

An explicit `tsgo` selection continues to require a compatible Microsoft compiler. An explicit `tsrs.moduleUrl` or native executable path remains an advanced override. A `tsgo` options object alone is not a new generator selector; preserve upstream semantics and migrate old boolean selectors through the existing migration rules.

Native eligibility requires a real tsconfig, a supported platform, and the required `async`/`fs` exports and `Program.prototype.getDeclarationEmit` capability. Check capabilities as well as versions. Classify unsupported options before creating build outputs. Do not retry a failing type check, compiler crash, or declaration emit through a different compiler: that can change published types and conceal errors.

### Compatibility limits

The native generators currently read program options from the on-disk tsconfig. They do not apply `dts.compilerOptions` or `dts.tsconfigRaw` to the compiler program. Declaration maps also require `compilerOptions.declarationMap` in that file. Preserve an existing classic compiler when these overrides matter; reject an incompatible explicit native selection with a migration diagnostic instead of silently ignoring settings. [Native option definitions](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.1/src/options.ts).

Retain TypeScript 5/6 for Vue/Volar and classic API consumers. Treat non-Volar custom languages as explicit native configurations that require `vfs` and filename mapping. Preserve `runExternalCode: false` by default. Content mappers require an explicit opt-in and cannot combine with native VFS in the current plugin. TypeScript 7.1's new API does not make old compiler API plugins interchangeable. [Plugin integrations](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.1/README.md), [TypeScript 7.1 API](https://devblogs.microsoft.com/typescript/announcing-typescript-7-1-beta/).

Native support makes the pinned compiler's language and library features available. It does not change `target`, `lib`, module resolution, runtime polyfills, or editor settings. Check syntax that also passes through Rolldown/Oxc, including import attributes, through the full pack pipeline before advertising support. A library that uses newer declaration syntax may need a higher minimum TypeScript version for its consumers; migration cannot downlevel arbitrary public types.

## Packaging and implementation

Keep ownership in the local packages. The global CLI continues to delegate to the project's `vite-plus`; it does not provide the compiler.

1. Add the same exact `tsrs` production dependency to core that #3352 adds to the CLI. Keep the CLI's direct dependency for lint resolution. Package managers can deduplicate matching versions, but neither package may rely on hoisting to the other. Avoid shipping a second Go compiler as a default fallback.
2. Keep `tsrs` and its API subpaths external when rebuilding tsdown. Resolve the default `tsrs.moduleUrl` from core's own dependency. Preserve tsrs's SDK, platform binary, and adjacent standard libraries. The lint executable path is not a declaration API module URL.
3. Update `mergePackageJson()` so upstream peer merging cannot replace the direct tsrs dependency with an optional peer. Keep `typescript` and the framework peers optional. Include TypeScript 7.1 prereleases in the effective peer range: tsdown's broad stable range alone does not admit `7.1.0-beta`.
4. Audit the declaration plugin's new `yuku` dependency and its native packages. The existing build externalizes `yuku-parser@0.19.0`; plugin `0.30.1` uses `yuku@^0.27.0`. Replace the old packaging assumption with the resolved dependency graph and verify native loading from the published tarball. [Plugin dependencies](https://registry.npmjs.org/rolldown-plugin-dts/0.30.1).
5. Put compiler selection in one core pack adapter, used by CLI resolution and public `build`/`resolveUserConfig` entry points. Preserve the remaining `vite-plus/pack` exports and custom generators. Hook after effective package/tsconfig discovery and before declaration-plugin construction, rather than setting a generator once in `pack-bin.ts` before defaults are known.
6. Resolve explicitly selected project TypeScript from the owning package. The plugin's classic compiler loader currently has a module-global cache. Scope compiler contexts by resolved compiler URL and project, or obtain upstream provider injection before claiming mixed-version programmatic builds work. Do not change process-wide module resolution to switch compilers. [Classic compiler loading](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.1/src/require.ts).
7. Preserve the declaration externalization rules for `vite-plus`, Vitest, PostCSS, and Lightning CSS, including when declarations are enabled by inference. Keep public `vite-plus/test*` specifiers in output. Verify ESM/CJS filenames, exports, maps, and ambient declarations against the existing pack fixtures.
8. Handle the expected native-backend notice without failing a successful build. `vp pack` defaults to `failOnWarn: true`; the plugin's experimental warning currently sets exit status 1 even when emit succeeds. Give that notice a structured upstream code and report it once as informational in the experimental Vite+ release. If a temporary adapter is needed, match only the pinned, known notice. Preserve failures for other warnings, including ignored compiler settings and missing maps.

Prefer an upstream integration hook for the adapter. If a local bundle transform is necessary, anchor it to the pinned source shape, assert the expected replacement count, and cover both CLI and programmatic calls. Do not add another public `native: true` switch alongside `dts.generator`.

Expose the declaration plugin version, bundled tsrs version, and TypeScript source version through toolchain metadata. Debug pack output should include the selected generator, compiler location, and selection reason. Check and pack can use different compilers when a project retains TypeScript; report those identities without claiming diagnostic equivalence.

## `vp migrate`

Extend the existing pack migration in [`crates/vp_migration/src/pack_config.rs`](../crates/vp_migration/src/pack_config.rs) and its TypeScript orchestration. Run it for existing Vite+ projects without requiring `--full`, including workspace packages and retained `tsdown.config.*` imports. Read the installed source toolchain before the final install, as the current versioned migration does.

Separate schema updates from compiler adoption. Apply static, behavior-preserving schema changes on the normal upgrade path. Keep TypeScript 5/6/7 selection and dependencies unless a recognized obsolete native configuration requires a change. Unknown source versions take the conservative path; do not assume a fresh project.

| Existing configuration | Automatic action | Boundary |
| --- | --- | --- |
| Top-level `dts.vue`, `build`, `parallel`, `eager`, `newContext` | Move them under `dts.tsc` and retain classic compiler selection where required. | Preserve explicit nested values; conflicting values or dynamic objects require a finding. |
| Top-level `dts.incremental` | Move `true`; for `false`, preserve the old behavior of following tsconfig rather than writing the new disabling override. | Conflicts with `dts.tsc.incremental` need review. Do not implement this as a blind key rename. |
| `dts.emitDtsOnly` / `dts.dtsInput` | Normalize equivalent static settings to `dts.dtsOnly`. | Conflicting aliases, or combination with `dts.entry`, need a resolved entry plan before editing. |
| `dts.entry` whose matches are already build entries | Keep the filter; preserve its old base directory when different from the build's `cwd`. | For static workspace paths, rewrite patterns relative to the resolved declaration cwd. Do not change compiler-config discovery by changing `dts.cwd` without checking it. |
| `dts.entry` that used to add declaration-only files | Add a separate declaration-only pack configuration with those inputs, and retain the original JS entry set. | Preserve names, output paths, maps, cleaning, and package exports. Dynamic globs/layouts or output collisions require review; never add JS outputs just to recover `.d.ts` files. |
| Explicit `tsgo` backed only by `@typescript/native-preview` | For supported plain TS projects with no custom executable/options, select bundled `tsrs`; preserve the other project compiler and type-check settings. | Run native eligibility checks. Keep the original config and dependencies when checks fail; report the blocker. |
| TypeScript 7.0 automatic/explicit `tsgo` | Keep the working `0.30.1` CLI bridge and identify it in the migration report. | Do not force a project-wide beta compiler upgrade. Native adoption remains an explicit `generator: 'tsrs'` change. |
| Custom `tsgo.path` | Report the changed contract: on 7.1 it selects an API server executable. | Do not copy a CLI binary path into `tsrs.path` or assume API compatibility. |
| Native compiler overrides or declaration maps configured only in `dts` | Create a project-owned pack tsconfig extending the original; set equivalent compiler options there and point only the affected pack config at it. | Move only static values, preserve effective `include`/`exclude`/`files`, references, relative paths, and project ownership. Keep shared configs unchanged. |

The `dts.entry` and alias changes come from [tsdown 0.23.7](https://github.com/rolldown/tsdown/releases/tag/v0.23.7). The native-preview removal, executable-path change, and native map behavior come from [plugin 0.29](https://github.com/sxzz/rolldown-plugin-dts/releases/tag/v0.29.0). Preserve deprecated options through the compatibility window where the target plugin still accepts them; use migration to remove warnings and handle semantic changes.

For a declaration-only split, generate a second `pack` array item using `dts: { dtsOnly: true }`, with no `dts.entry`. Run both through tsdown's existing shared cleanup in `buildWithConfigs`, and avoid competing export writes. Verify that watch cleanup retains each sibling's outputs. Only automate the split when the migrator can preserve the output contract. Otherwise report the exact missing declaration entries and retain the source configuration for review. [Shared cleanup implementation](https://github.com/rolldown/tsdown/blob/v0.23.7/src/build.ts).

For a native-preview migration, remove that dependency only after checking scripts, imports, CI, shared workspace consumers, and package-manager catalogs. Retain standalone `tsgo` tasks that still use it. Do not add `typescript` to the generic bundled-package removal list, alias `typescript` to `tsrs`, or replace `tsc -b`/`tsc --emitDeclarationOnly` with `vp check`.

Preserve comments, JSONC, workspace ownership, and explicit values. Use static AST transforms; do not evaluate a config to decide what to rewrite. Spreads, computed keys, conflicting aliases, unresolved package versions, and third-party config files produce actionable findings. Each finding names the file, unsupported setting, and next action. Running migration twice must produce no second diff. Stop a dependent rewrite if its prerequisite fails; do not leave a partially converted native configuration.

Treat a configuration that the target toolchain cannot run as an upgrade blocker, not a successful migration with a warning. Identify it before the final dependency install and retain the previous toolchain when possible. If the user has already upgraded, report the incomplete migration and the supported rollback or manual edit. Keep harmless deprecations separate from these blockers in the migration report.

For TypeScript 7 incompatibilities such as removed compiler options, retain the classic generator where possible. Report source-level type errors and public declaration incompatibilities for review. Do not weaken checking with `skipLibCheck`, `noCheck`, or disabled rules to make migration appear successful.

## Release gates

| Area | Required evidence |
| --- | --- |
| No project compiler | Published CLI/core tarballs emit inferred declarations with no `typescript`, native-preview, or project `tsrs`; the native standard libraries resolve. |
| Existing compilers | TypeScript 5.9, 6.0, 7.0, and 7.1 beta retain the intended generator. Explicit strings, custom generators, missing/corrupt compilers, and Oxc conflicts produce the documented outcomes. |
| Packaging | npm, pnpm with `hoist: false`, Yarn PnP, and Bun; workspace and standalone installs; local CLI with no global `vp`; public config types with declaration checking enabled. |
| Declarations | Compare old/new exports, `.d.ts`/`.d.mts`/`.d.cts`, maps, JSDoc, JS inputs, referenced projects, entry filters, and retained public imports. Compile representative consumer fixtures with the library's supported TS5/6/7 versions. |
| Native parity | Compare tsrs with exact `typescript@7.1.0-beta` using identical options and inputs. Compare older compilers separately to identify version changes rather than mislabeling them as Rust defects. Assert real process exit codes, including the default `failOnWarn` behavior. |
| Configuration | Arrays, callbacks, shared configs, `cwd`, disabled/inferred dts, CLI flags, inline compiler overrides, no tsconfig, Vue/Volar, VFS, and content-mapper opt-in. |
| Lifecycle | Watch edit/add/delete/config changes, failed rebuild recovery, cancellation, multiple projects/compiler versions, API cleanup, and no stale declarations or leftover native processes. |
| Migration | Existing Vite+ without `--full`, all supported package managers, catalogs, retained consumers, source-version detection, static rewrites, manual findings, and idempotence. |
| Platforms | macOS x64/ARM64, Linux x64/ARM64 glibc/musl, Windows x64; Windows ARM64 remains a blocker inherited from #3352 until tsrs ships and passes native validation. |

Use existing [`command_pack_dts_compat`](../crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/command_pack_dts_compat/snapshots.toml), pack package-manager/watch fixtures, [`pack.spec.ts`](../packages/cli/src/__tests__/pack.spec.ts), and migration tests. Add fixtures to the snapshot runner with the local registry. The native-preview fixture currently proves a supported path and must become an explicit migration regression rather than disappear.

Measure cold emit, warm rebuilds, and peak memory on the same projects and compiler settings. Lint benchmarks from #3352 do not establish declaration performance or correctness. Keep tsrs's reference defaults; do not enable its fast preset as part of this change.

## Delivery order

1. Qualify the Vite/Rolldown combination and update tsdown, its extensions, declaration plugin, native dependencies, and core engine/peer metadata. Retain supported legacy compiler paths.
2. Add the shared pack adapter, native dependency resolution, toolchain reporting, and the no-TypeScript path. Keep this in the experimental release until its gates pass.
3. Add the versioned migration transformations and manual findings before releasing the upgrade to existing projects. Include the native-preview and `dts.entry` transitions.
4. Update pack documentation and plain TypeScript templates after validation. Omit TypeScript from new templates only when their scripts, editor setup, and framework dependencies permit it. Retain it in templates that need the classic API.

Keep these as small PRs stacked on #3352. Do not make a stable release until Windows ARM64 and the shared Rolldown integration have a supported outcome. The fallback release plan is to retain the previous qualified pack toolchain while completing these prerequisites, rather than introduce an untested compiler fallback.

## Design probe

On macOS ARM64 with Node `24.22.2`, an isolated npm installation of `tsdown@0.23.7`, `rolldown-plugin-dts@0.30.1`, and `tsrs@0.24.0-ts7.1.0-beta` confirmed these behaviors:

- `typescript` could not resolve, and upstream `dts: true` failed with the missing-TypeScript diagnostic.
- Explicit `generator: 'tsrs'` emitted `export declare const answer: number;` without installing TypeScript.
- Explicit `typecheck: true` rejected a string assigned to that numeric export.
- With `failOnWarn: true`, emit completed and `build()` resolved, but the process exited with status 1 because of the experimental-backend warning. The adapter must handle this before the native default can ship.

The probe used published packages from npm because the question depends on their shipped native API. Installation disabled lifecycle scripts and automatic peer installation. This checks the upstream integration premise only; it does not validate the proposed Vite+ adapter, migration, cross-platform support, or release performance.

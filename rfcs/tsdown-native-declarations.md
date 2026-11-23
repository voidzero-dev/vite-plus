# RFC: Native declaration generation for `vp pack`

Status: Proposed, experimental. Researched on 2026-11-19 against `main` at `1fbe723e9`. Updated on 2026-11-23 for [#3511](https://github.com/voidzero-dev/vite-plus/pull/3511).

## Decision

When a `vp pack` build needs declarations and the project does not provide `typescript`, generate them with the `tsrs` package that `vite-plus` already ships for `vp lint` and `vp check`. In all other cases, keep the generator that tsdown and `rolldown-plugin-dts` (the declaration plugin) select today.

Today this case fails with "TypeScript is not installed". The change turns that error into a working build. It does not change the output of a build that works today, because every other selection stays upstream.

The declaration plugin never selects `tsrs` automatically, even when it is installed. Its documentation says so, and the probe below confirms it. Vite+ therefore owns this default. [Generator option](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.2/src/options.ts#L40-L58)

The change does not replace `tsc` tasks, the editor's compiler, Vue/Volar integrations, or tools that use the classic TypeScript API. It does not remove `typescript` from existing projects. It does not change the default of the programmatic `build()` from `vite-plus/pack`. Upstream marks `tsrs` as highly experimental, and TypeScript 7.1 is still a beta. The default therefore ships as experimental.

## Baseline

| Component                                 | `main`                        | After [#3511](https://github.com/voidzero-dev/vite-plus/pull/3511) |
| ----------------------------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `tsdown`, `@tsdown/css`, `@tsdown/exe`    | `0.23.4`                      | `0.23.7`                                                           |
| Declaration plugin bundled through tsdown | `0.28.8`, no `tsrs` generator | `0.30.2`. The `tsrs` generator first ships in `0.29.0`.            |
| `tsrs`, a CLI dependency used by lint     | `=7.1.0-beta.tsrs4`           | Unchanged, and also used by pack                                   |
| Bundled Rolldown                          | `1.2.19`                      | `1.3.3`, which tsdown `0.23.7` requires (`~1.3.0`)                 |
| Bundled Vite                              | `8.3.8`                       | `8.4.0-beta.1`                                                     |

[#3511](https://github.com/voidzero-dev/vite-plus/pull/3511) performs this upgrade. It is a prerequisite, not part of this design, and this change merges after it. See [Out of scope](#out-of-scope) for the upstream changes that it handles and the ones that stay open.

tsdown `0.23.7` and the declaration plugin `0.30.2` require Node `^22.18.0 || ^24.11.0 || >=26.0.0`, the same range as `packages/cli`. `packages/core` still advertises Node 20, but its bundled tsdown `0.23.4` already requires `^22.18.0`. Fix that metadata separately.

### `tsrs` version coupling

The declaration plugin supports "only the latest `tsrs` release", but its own repository pins `tsrs@0.24.0-ts7.1.0-beta` at `v0.30.2`. Vite+ updates `tsrs` on its own schedule for lint. One `tsrs` version serves lint and pack. [Plugin tsrs pin](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.2/pnpm-workspace.yaml#L35)

- Treat each `tsrs` update and each declaration-plugin update as a pack change.
- The upgrade-deps PR must pass the declaration fixtures below.
- If an update breaks declaration emit, hold the update for both lint and pack. Do not ship a second `tsrs` for pack.

## Selection rule

Apply the rule to each resolved config in `packages/cli/src/pack-bin.ts`, after `resolveUserConfig()` and before `buildWithConfigs()`. At that point tsdown has resolved declaration enablement, including inference from `types`, `typings`, `exports.types`, and `compilerOptions.declaration`. The rule runs again on watch reloads because `runBuild` resolves the configs again. [tsdown dts inference](https://github.com/rolldown/tsdown/blob/v0.23.7/src/config/options.ts#L154-L166)

Select `generator: 'tsrs'` only when all of these conditions are true:

1. `config.dts` is an object, so declarations are enabled.
2. The user did not set `dts.generator`.
3. The effective tsconfig does not set `compilerOptions.isolatedDeclarations`. Upstream selects Oxc there, which works without TypeScript.
4. The config does not use Vue or other custom languages. Vue and other Volar languages need the classic API, and upstream reports that requirement. Other custom languages need `tsrs.vfs`, which only the user can opt into.
5. `typescript` does not resolve from the declaration plugin's location.

Condition 5 uses the same lookup as the plugin, from the bundled tsdown in `@voidzero-dev/vite-plus-core`. It does not use the project root. The plugin loads the classic compiler with `createRequire(import.meta.url)` and the native API with `import.meta.resolve('typescript')`. A different origin can disagree with the plugin. [Classic loader](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.2/src/require.ts#L5), [native loader](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.2/src/tsgo.ts#L30-L42)

If all conditions are true but the config sets `dts.compilerOptions` or `dts.tsconfigRaw`, do not select `tsrs`. Fail with a diagnostic that names the options and two fixes: move them into a tsconfig, or install `typescript`. Native generators do not apply these options to the compiler program. In the probe, `dts.compilerOptions.stripInternal: true` had no effect and gave no warning. The build fails today in this case, so a clear error keeps the current result and does not publish different types.

If no tsconfig exists, do not select `tsrs`, because native generators require one. Keep the upstream error.

In every other case, leave the config unchanged. An explicit generator string or a custom `Generator` object stays as it is. Do not retry a failed check or emit with another compiler, because that can change published types and hide errors.

### Compiler location

Pass `dts.tsrs.moduleUrl` as the file URL of the CLI's own `tsrs`, from `resolveBundled('tsrs')` in `packages/cli/src/utils/constants.ts`. Do this when Vite+ selects `tsrs`, and also when the user selects `generator: 'tsrs'` without a `moduleUrl`. A user value for `dts.tsrs.moduleUrl` stays as it is.

The plugin loads `tsrs/async` relative to that URL. Lint and pack then use the same binary, and `@voidzero-dev/vite-plus-core` needs no new dependency. Without this URL, the plugin's default `import.meta.resolve('tsrs')` from core can fail in strict layouts, because core does not depend on `tsrs`.

### Hoisting

Condition 5 depends on the install layout. A transitive `typescript` that resolves from core, for example through npm's flat layout or pnpm's hidden hoisting, makes upstream select `tsc` or `tsgo`. That is unchanged upstream behavior, but the generator for one project can change when its dependency tree changes. This pnpm behavior is expected, not tested.

- When Vite+ selects `tsrs`, print one info line per build with the generator, the `tsrs` version, and the reason.
- Print the `typescript` path that was found in debug output.
- Document explicit `generator: 'tsrs'` for projects that need a fixed generator.

## Experimental warning

The declaration plugin warns one time per process: "tsrs support is highly experimental and may change or break without notice." With `failOnWarn: true`, or `'ci-only'` in CI, this warning makes the process exit with status 1 after a successful emit.

`vp pack` itself defaults to `failOnWarn: false`, the tsdown default. The `--fail-on-warn` help text says `default: true`, which is a separate bug. The CLI command sets `ignoreOptionDefaultValue`, so that default is not applied.

When the user did not set `dts.generator`, add the exact warning to `suppressWarnings` before `resolveUserConfig()`. Keep the user's entries. tsdown builds its logger during resolution, so the entry cannot be added later. The warning fires only when `tsrs` runs, so the entry has no effect when Vite+ does not select `tsrs`. The info line above replaces the warning. When the user selects `tsrs` explicitly, keep the upstream warning. [tsdown suppressWarnings](https://github.com/rolldown/tsdown/blob/v0.23.7/src/config/types.ts#L495-L507)

## Packaging

The local `vite-plus` package owns this behavior. The global CLI continues to delegate to the project's `vite-plus`, and it does not supply a compiler.

1. Keep the `tsrs` dependency in `packages/cli` only. Do not add it to core.
2. Keep the optional `tsrs: "*"` peer that #3511 declares in core. It serves direct core users who select `tsrs` programmatically.
3. Show the declaration plugin version and the TypeScript version that `tsrs` targets in toolchain metadata, next to the existing `tsrs` entry.

## Compatibility limits

- Native generators read program options only from the tsconfig file on disk. Declaration maps also need `compilerOptions.declarationMap` in that file. [Native option notes](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.2/src/options.ts#L100-L122)
- Vue/Volar and classic API consumers keep TypeScript 5/6.
- Non-Volar custom languages need `tsrs.vfs`. Content mappers need the `runExternalCode` opt-in and cannot combine with VFS. Keep `runExternalCode: false`.
- `tsrs` follows TypeScript 7.1 beta. Diagnostics and emitted declarations can differ from TypeScript 5/6. A library that uses newer declaration syntax can need a newer TypeScript in its consumers.
- The rule does not change `target`, `lib`, module resolution, polyfills, or editor settings.

## Release gates

| Area                | Required evidence                                                                                                                                                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No project compiler | Packed CLI and core tarballs emit inferred and explicit declarations with no `typescript` and no project `tsrs`. Check npm, pnpm with default hoisting and `hoist: false`, Yarn PnP, and Bun.                                         |
| Selection           | No TypeScript selects `tsrs`. TypeScript 5.9 and 6.0 keep `tsc`. TypeScript 7.1 beta keeps `tsgo`. `isolatedDeclarations` keeps Oxc. An explicit generator stays. `dts.compilerOptions` without TypeScript fails with the diagnostic. |
| Exit codes          | Automatic `tsrs` with `failOnWarn: true` exits 0. Explicit `tsrs` keeps the upstream warning. Type errors with `typecheck: true` still fail.                                                                                          |
| Lifecycle           | Watch rebuilds after edits and config changes. No `tsrs` process remains after exit or cancellation.                                                                                                                                  |
| Platforms           | macOS x64/ARM64, Linux x64/ARM64 glibc and musl, Windows x64/ARM64. `tsrs` ships a package for each of these eight targets. Check `yuku` on musl.                                                                                     |
| Updates             | The upgrade-deps PR runs the declaration fixtures for each `tsrs` or declaration-plugin update.                                                                                                                                       |

Add the cases to [`command_pack_dts_compat`](../crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/command_pack_dts_compat/snapshots.toml) with the local registry, and add unit coverage for the rule in [`pack.spec.ts`](../packages/cli/src/__tests__/pack.spec.ts).

The experimental release does not need performance or memory benchmarks, parity runs against TypeScript 5/6, VFS or content-mapper coverage, or several compiler versions in one process.

## Upstream requests

1. tsdown: allow changes to `dts` in `tsdownConfigResolved`, or add a hook before the declaration plugin starts. The programmatic `build()` can then get the same default without the CLI seam. Today the hook documentation says that changes "are not supported". [tsdown plugin hooks](https://github.com/rolldown/tsdown/blob/v0.23.7/src/features/plugin.ts#L40-L51)
2. Declaration plugin: test against the latest `tsrs` release, or declare a supported `tsrs` range.
3. Declaration plugin: give the experimental warning a stable code, so suppression does not depend on its text.

## Out of scope

- **Dependency upgrade.** [#3511](https://github.com/voidzero-dev/vite-plus/pull/3511) upgrades tsdown, the declaration plugin, Rolldown, and Vite. It handles these upstream changes:
  - `@typescript/native-preview` is no longer supported ([plugin 0.29.0](https://github.com/sxzz/rolldown-plugin-dts/releases/tag/v0.29.0)). #3511 removes its core peer and development dependency. `vp migrate` moves TypeScript 5/6 packages that select it to `typescript@7.1.0-beta`, not to `tsrs`. The `command_pack_dts_native_preview` snapshot case becomes `command_pack_dts_typescript_5` and the `migration_pack_native_preview` regression.
  - `tsgo.path` selects an API server executable on TypeScript 7.1. `vp migrate` reports a custom `tsgo.path` for review.
  - The merged `typescript` peer range in `mergePackageJson()` admits the plugin's TypeScript 7.1 prereleases.
  - The plugin depends on `yuku` instead of `yuku-parser@0.19.0`. Core keeps `yuku@0.28.1` external as a dependency.
- **Upgrade follow-ups.** These upstream changes stay open after #3511:
  - `dts.entry` only filters build entries, and a matching file that is not an entry no longer gets a declaration. Its patterns resolve from tsdown's `cwd`. `dts.emitDtsOnly` and `dts.dtsInput` are deprecated aliases of `dts.dtsOnly`. ([tsdown 0.23.7](https://github.com/rolldown/tsdown/releases/tag/v0.23.7))
  - Top-level `dts.vue`, `build`, `incremental`, `parallel`, `eager`, and `newContext` are deprecated. The plugin moves them under `dts.tsc` at runtime with a warning, so migration only needs to remove warnings. [Plugin deprecations](https://github.com/sxzz/rolldown-plugin-dts/blob/v0.30.2/src/options.ts#L411-L435)
  - `@yuku-toolchain/*` publishes no musl package, so check `yuku` loading on musl.
- **Migration.** Rewrites that change the output layout, such as a separate declaration-only pack config or a generated pack tsconfig, start as report-only findings.
- **Templates.** Remove `typescript` from plain TypeScript templates only after the gates pass, and only when their scripts, editor setup, and framework dependencies permit it.
- **Programmatic default** for `build()` from `vite-plus/pack`, until upstream request 1 lands.
- **Several TypeScript versions in one process.** The plugin caches one classic compiler per process.

## Design probe

On macOS ARM64 with Node `24.22.3`, isolated npm installs of `tsdown@0.23.7`, `rolldown-plugin-dts@0.30.2`, and `rolldown@1.3.3` gave these results. The installs disabled lifecycle scripts and automatic peers.

| Case                                                                                                     | Result                                                                                                            |
| -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `dts: true`, no `typescript`                                                                             | Fails: "TypeScript is not installed".                                                                             |
| `dts: true`, `tsrs` installed, no `typescript`                                                           | Same failure. Upstream does not select `tsrs`.                                                                    |
| `isolatedDeclarations`, no `typescript`                                                                  | Oxc emits declarations.                                                                                           |
| Explicit `tsrs`, with `7.1.0-beta.tsrs2`, `7.1.0-beta.tsrs3`, and `7.1.0-beta.tsrs4`                     | Emits declarations, JSDoc, and declaration maps.                                                                  |
| `tsrs.moduleUrl` that points to a separate install, with no `tsrs` near the plugin                       | Emits declarations.                                                                                               |
| `typecheck: true` with a type error                                                                      | The build fails.                                                                                                  |
| `dts.compilerOptions.stripInternal: true`                                                                | Ignored without a warning. The `@internal` export stays.                                                          |
| `failOnWarn: true`                                                                                       | Emits, then exits with status 1 because of the experimental warning.                                              |
| `failOnWarn: true` with the warning in `suppressWarnings`                                                | Exits 0.                                                                                                          |
| `resolveUserConfig()`, then set the generator, then `buildWithConfigs()`, with dts inferred from `types` | Resolved `dts` is `{}`. The build emits declarations and exits 0 with the suppression. No `tsrs` process remains. |

These results check the upstream premises and the CLI seam. They do not validate the Vite+ implementation, other package managers, or other platforms.

# Ecosystem correctness comparison

The default `tsrs@0.22.0-ts7.1.0-beta` switch is not diagnostically equivalent to
`oxlint-tsgolint@7.0.2005`. The representative sweep found new type-check errors in
Hono and Vue Core, and a DOM-library compatibility change in OneSignal. These
findings prevent a regression-free release claim.

On 2026-11-06, a Linux x64/glibc Codespace ran 141 commands across 14 pinned forks
from the [ecosystem catalog](https://github.com/vite-plus-ecosystem-ci/.github).
The explicit type-check workload covered 2,289 files. No command timed out or
terminated by a signal. Seven repositories had identical normalized diagnostics;
seven had differences. This covers the selected repositories, not all 83 catalog entries.

## Controls and method

The comparison used three toolchains:

| Control                 | Published Vite+ package                                 | Native lint backend        |
| ----------------------- | ------------------------------------------------------- | -------------------------- |
| Stable                  | `1.5.0`                                                 | `oxlint-tsgolint@7.0.2005` |
| Preview with Go control | `0.0.0-commit.4fcf99edbe42fe3305dcd41dc884a4c4b7f5fd45` | `oxlint-tsgolint@7.0.2005` |
| Default preview         | Same preview                                            | `tsrs@0.22.0-ts7.1.0-beta` |

All controls used Oxlint `1.91.0`. Stable and preview with Go produced identical
normalized lint and TypeScript diagnostics in every selected repository. This
control separates backend differences from other preview changes.

Each project retained its original frozen lockfile, application dependencies, and
source configuration. The runner temporarily linked the published CLI, Vite,
Oxlint, and Oxfmt into workspace importers, then restored those links. Tracked
source files remained unchanged. The setup used each project's pinned package
manager, with lifecycle scripts disabled. Node.js was `24.22.2`, except
`eslint-plugin-oxlint` and `react-data-grid`, which used `26.13.0` to meet their engine requirements.

Each control ran these commands:

```sh
vp lint --format json --threads 4
vp lint --type-aware --type-check --format json --threads 4
vp check
```

Hono retained its lint scopes: `src`, `runtime-tests`, `build`, `perf-measures`,
`benchmarks`, and `adapters`. Hono and Vue Core do not enable type-aware linting in
their current configurations; the explicit flags add stress coverage. React Data
Grid enables type-aware rules but does not enable TypeScript diagnostics by default.

The runner also used `vp run --no-cache` for five existing project tasks:
`oxc-walker`'s `test:types`, `urllib`'s `typecheck`, `media-processors`'s `typecheck`,
`react-data-grid`'s `typecheck`, and `vue-core`'s `check`. These tasks retain their
own compiler selection. A `tsc` task does not automatically become a `tsrs` task.

The runner compared sorted JSON diagnostics, including codes, severity, messages,
files, spans, and help text. It normalized absolute project paths. Process traces
confirmed the expected native backend in all 42 explicit type-check runs. React
Data Grid and Vue Core required the matching optional `@vitest/browser-playwright@5.1.0`
peer for configuration imports. No browser installation was required.

## Repository results

The counts below include errors and warnings from the explicit type-check workload.
Equal counts alone do not establish equality; the comparison also checked diagnostic content.
All source links point to immutable commits in `vite-plus-ecosystem-ci`.

| Repository / commit                                                                                                                                | Files | Go diagnostics | tsrs diagnostics | Comparison                                    |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ----: | -------------: | ---------------: | --------------------------------------------- |
| [oxc-walker `9e520f25d`](https://github.com/vite-plus-ecosystem-ci/oxc-walker/tree/9e520f25d2bf5c3748f51d143c534fedff207147)                       |    12 |              0 |                0 | Identical diagnostics                         |
| [eslint-plugin-oxlint `ca4b10897`](https://github.com/vite-plus-ecosystem-ci/eslint-plugin-oxlint/tree/ca4b1089757245fa9a6a6ee46341d94a218d9c05)   |    41 |              0 |                0 | Identical diagnostics                         |
| [solid-services `add4b036b`](https://github.com/vite-plus-ecosystem-ci/solid-services/tree/add4b036b72de7e6372b07faf0aed6b3f2e506d4)               |    10 |              0 |                0 | Identical diagnostics                         |
| [pinia-shared-state `1b1ec6109`](https://github.com/vite-plus-ecosystem-ci/pinia-shared-state/tree/1b1ec6109b6f46079e4ddb5aeb9feff1d7188c24)       |     6 |              0 |                0 | Identical diagnostics                         |
| [urllib `91f32d53f`](https://github.com/vite-plus-ecosystem-ci/urllib/tree/91f32d53faaae34a9f09f38e402e617420bc4d15)                               |    72 |              6 |                6 | Identical diagnostics                         |
| [anywidget `443d720b2`](https://github.com/vite-plus-ecosystem-ci/anywidget/tree/443d720b2233fff131a0f0a842b08744c8ffabd6)                         |    66 |             14 |               14 | Identical diagnostics                         |
| [css-modules-kit `634608fdd`](https://github.com/vite-plus-ecosystem-ci/css-modules-kit/tree/634608fdd401001d1fb05511a89177acf38b8111)             |   131 |              0 |                0 | Identical diagnostics                         |
| [media-processors `79c87cd5c`](https://github.com/vite-plus-ecosystem-ci/media-processors/tree/79c87cd5c066a0a9abea979044c38c69603a85e8)           |    22 |            101 |               91 | 10 fewer diagnostics                          |
| [web-app-template `f9a09f3d8`](https://github.com/vite-plus-ecosystem-ci/web-app-template/tree/f9a09f3d81002372e839247c6274af43177bbeca)           |     8 |              3 |                1 | 2 fewer diagnostics                           |
| [OneSignal-Website-SDK `5375640c0`](https://github.com/vite-plus-ecosystem-ci/OneSignal-Website-SDK/tree/5375640c001a2dc462ed1cb938520edfb9a23d07) |   298 |              8 |                9 | New DOM-library requirement                   |
| [react-data-grid `384aedffc`](https://github.com/vite-plus-ecosystem-ci/react-data-grid/tree/384aedffc2cac013666b58b6787d888b9dde3a94)             |   127 |            302 |              300 | 2 fewer diagnostics; message changes          |
| [vue-clamp `6f0da3991`](https://github.com/vite-plus-ecosystem-ci/vue-clamp/tree/6f0da3991f804587ba402a7ca22aec4564eab938)                         |   134 |              5 |                1 | 4 fewer recursion errors                      |
| [hono `c91b60a25`](https://github.com/vite-plus-ecosystem-ci/hono/tree/c91b60a251186fb024557c6d55a11d8e388f179c)                                   |   457 |            474 |              476 | 2 new contextual-typing errors; span changes  |
| [vue-core `747e8d668`](https://github.com/vite-plus-ecosystem-ci/vue-core/tree/747e8d668c4a64ae6bd9dcc74c173364f1c1490b)                           |   905 |            547 |              546 | 7 new source errors; 8 fewer recursion errors |

Default `vp lint` diagnostics matched in ten repositories. Its four differences
were OneSignal, media-processors, web-app-template, and vue-clamp. Vue Clamp's lint
exit changed from `1` to `0` after four `TS2321` errors disappeared; one warning remained.
The additional Hono, Vue Core, and React Data Grid differences appeared with explicit type-check flags.

`vp check` passed in oxc-walker, eslint-plugin-oxlint, pinia-shared-state, and
css-modules-kit under all controls. The other ten failed under all controls.
Formatting stopped five of them before lint: solid-services, anywidget,
vue-clamp, Hono, and Vue Core. These failures do not demonstrate backend failure
or backend coverage.

## Confirmed contextual-typing regression

[Hono's `Style` implementation](https://github.com/vite-plus-ecosystem-ci/hono/blob/c91b60a251186fb024557c6d55a11d8e388f179c/src/helper/css/index.ts#L192)
produces two new `TS7031` errors for `children` and `nonce` with tsrs. This reduced
source reproduces the failure with strict type checking:

```ts
interface StyleType {
  (args?: { children?: Promise<string>; nonce?: string }): string;
}
export const Style: StyleType = ({ children, nonce } = {}) => `${children}${nonce}`;
```

TypeScript `6.0.3`, TypeScript `7.0.2`, and both Go lint controls accept this source.
The default preview and standalone tsrs report both errors. An explicit parameter
type annotation makes all controls pass. Hono's production project also passes
`tsc -p tsconfig.build.json --noEmit` but fails the same check with standalone tsrs.
This confirms a compiler difference independent of Vite+ backend resolution.

[Vue Core's `applyTextModel`](https://github.com/vite-plus-ecosystem-ci/vue-core/blob/747e8d668c4a64ae6bd9dcc74c173364f1c1490b/packages/runtime-vapor/src/directives/vModel.ts#L46)
has the same default-destructuring pattern. Its `trim`, `number`, and `lazy` bindings
produce three new `TS7031` errors. A four-line reduction with a mapped optional
parameter passes both TypeScript versions and both Go controls, but fails both tsrs paths.

## Vue Core selector types

[Vue Core's scoped-style transformer](https://github.com/vite-plus-ecosystem-ci/vue-core/blob/747e8d668c4a64ae6bd9dcc74c173364f1c1490b/packages/compiler-sfc/src/style/pluginScoped.ts#L107)
produces a new `TS5115` circular-instantiation error and two `TS7006` callback errors.
This smaller case uses the project's locked `postcss-selector-parser@7.1.6`:

```ts
import type parser from 'postcss-selector-parser';
export function inspect(selector: parser.Selector): void {
  selector.each((n) => {
    if (n.type === 'pseudo') {
      n.nodes[0].each((ss) => {
        void ss;
      });
    }
  });
}
```

Both TypeScript versions and both Go controls accept the reduced source. The
default preview and standalone tsrs report `TS5115` and `TS7006`. The reduction
uses strict mode, bundler module resolution, and `skipLibCheck: true`.

## Vue Core generic conditional return type

[Vue Core's `toHandlerKey`](https://github.com/vite-plus-ecosystem-ci/vue-core/blob/747e8d668c4a64ae6bd9dcc74c173364f1c1490b/packages/shared/src/general.ts#L147)
produces a new `TS2322` when it assigns a generic function with a conditional
return type. Both an isolated copy of the original implementation and this
reduction reproduce the error:

```ts
declare function cache<T extends (s: string) => string>(fn: T): T;
export const handler: <T extends string>(s: T) => T extends '' ? '' : `on${Capitalize<T>}` = cache(
  <T extends string>(s: T) => {
    const value = s ? `on${s}` : '';
    return value as T extends '' ? '' : `on${Capitalize<T>}`;
  },
);
```

Both TypeScript versions and both Go controls accept both cases. The default
preview and standalone tsrs reject both cases with `TS2322`. The reduction uses
the same strict compiler options as the selector case.

## OneSignal DOM-library difference

[OneSignal's notification mock](https://github.com/vite-plus-ecosystem-ci/OneSignal-Website-SDK/blob/5375640c001a2dc462ed1cb938520edfb9a23d07/__test__/support/environment/TestEnvironmentHelpers.ts#L46)
produces a new `TS2741`: the constructor lacks `Notification.maxActions`. The tsrs
`lib.dom.d.ts` requires that static property. The tested TypeScript `6.0.3` and
`7.0.2` libraries do not contain it.

```ts
type HasMaxActions = 'maxActions' extends keyof typeof Notification ? true : false;
export const hasMaxActions: HasMaxActions = false;
```

The TypeScript and Go controls accept this probe. Both tsrs paths report `TS2322`.
This establishes a library-contract difference; it does not establish that the
new DOM declaration is incorrect. The difference affects OneSignal's existing
configuration because it already enables TypeScript diagnostics.

## Other differences and limits

Media-processors, web-app-template, vue-clamp, and React Data Grid report fewer
recursive-comparison or configuration-compatibility errors with tsrs. Vue Core
also loses eight `TS2321` errors but adds seven production-source errors. Its total
therefore falls from 547 to 546 despite the new failures. Fewer diagnostics are
not classified as fixes without a separate correctness check.

React Data Grid also changes expanded type text in twelve existing diagnostics.
Vue Core changes two existing messages, and Hono moves three existing `TS2875`
spans. These presentation changes are included in the comparison.

Several errors are shared by both backends. They include urllib's six
`Http2Stream.respond` errors, anywidget's 14 type errors, missing generated
`video-track-processor` declarations in media-processors, and a missing translation
file in web-app-template. The broad Hono workload includes optional benchmark
and runtime fixtures. React Data Grid and Vue Core include test declarations
that conflict with the overlaid toolchain's Vitest types.

Only oxc-walker's existing type-check task passed. The other four tasks failed
under every control, with matching TypeScript diagnostics after installation-path
normalization. Vue Core initially returned `2` and then `1` as its incremental
cache warmed; fresh build-info files produced exit `2` and identical output in
all three controls. No task compiler was replaced by the default lint backend.

This method isolates the backend while preserving application dependencies. It
does not reproduce each repository's full build pipeline or a complete dependency
upgrade. Shared setup and source errors must not be counted as new tsrs regressions.
The reduced cases provide independent evidence for the new compiler failures.

The primary sweep used Codespaces job `job-d8ac88c64fc64b11`. Final anywidget,
React Data Grid, and Vue Core results came from `job-f5f12efe4b514e83`, which also
reduced the Hono and OneSignal cases. Vue reductions used `job-cf53f53e0a2d4216` and `job-120ca876783b4213`.
The harness, frozen dependency receipts, process traces, compiler outputs, and
raw JSON remain outside this repository.

# RFC: Replace tsgolint with tsrs-lint

Status: Proposed for the next release.

## User experience

Upgrading `vite-plus` makes tsrs the default backend for type-aware linting and
type checking in `vp lint` and `vp check`. Projects with these checks enabled
can benefit from tsrs performance improvements without additional setup.
Users keep their existing commands and configuration; no separate `tsrs`
installation is required.

## Design

Oxlint remains the linter frontend. The local `vite-plus` package bundles
`tsrs-lint` and selects it through the shared
[`resolve-lint.ts`](../packages/cli/src/resolve-lint.ts) resolver. Version output
and toolchain metadata identify `tsrs`.

## Dependency changes

| Dependency                | Before                                      | After                                      |
| ------------------------- | ------------------------------------------- | ------------------------------------------ |
| Linter frontend           | `oxlint@1.93.0`                             | Unchanged                                  |
| Production backend        | `oxlint-tsgolint@7.0.2006`                  | `tsrs@0.25.0-ts7.1.0-beta`                 |
| Development-only backend  | None                                        | `oxlint-tsgolint@7.0.2006` for comparisons |
| Backend platform packages | `@oxlint-tsgolint/*`: six optional packages | `@tsrs/*`: eight optional packages         |

Production installs use tsrs; repository development retains Go for comparisons.
The project's `typescript` dependency and standalone `tsc` tasks are unchanged.

## Compatibility and release considerations

- tsrs targets TypeScript `7.1.0-beta`, so compiler diagnostics can change.
- With `lint.options.typeCheck` enabled, tsrs checks whole TypeScript programs,
  including files excluded by lint ignore patterns. Existing projects can
  report additional errors. Projects control compiler scope through `tsconfig.json`.
- tsrs `0.25.0` provides native packages for all Vite+ targets, including Windows ARM64.
  The same backend runs on every supported platform.

The [benchmark report](../bench/tsrs-lint.md) contains historical timing, memory,
and disk-usage measurements. Refresh them for the release candidate.

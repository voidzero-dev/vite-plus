# RFC: Replace tsgolint with tsrs-lint

Status: Proposed; experimental default in this draft.

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
| Production backend        | `oxlint-tsgolint@7.0.2006`                  | `tsrs@0.24.0-ts7.1.0-beta`                 |
| Development-only backend  | None                                        | `oxlint-tsgolint@7.0.2006` for comparisons |
| Backend platform packages | `@oxlint-tsgolint/*`: six optional packages | `@tsrs/*`: seven optional packages         |

Production installs use tsrs; repository development retains Go for comparisons.
The project's `typescript` dependency and standalone `tsc` tasks are unchanged.

## Compatibility and release considerations

- tsrs targets TypeScript `7.1.0-beta`, so compiler diagnostics can change.
- With `lint.options.typeCheck` enabled, tsrs checks whole TypeScript programs,
  including files excluded by lint ignore patterns. Existing projects can
  report additional errors.
- Windows ARM64 remains a release blocker: tsrs has no binary for that target,
  and this integration has no automatic Go fallback.

The [benchmark report](../bench/tsrs-lint.md) contains historical timing, memory,
and disk-usage measurements. Refresh them for the release candidate.

# RFC: Replace tsgolint with tsrs-lint

- Status: Proposed; experimental default in this draft
- Related: [check command](./check-command.md), [toolchain metadata](./toolchain-command.md)

## Design

Replace the production dependency on `oxlint-tsgolint` with an exact `tsrs` pin,
currently `0.24.0-ts7.1.0-beta`. Keep the Go package as a development dependency
for comparisons. Oxlint remains the linter frontend and delegates type-aware
rules and optional TypeScript diagnostics to the bundled `tsrs-lint` executable.

The local `vite-plus` package owns this integration. Its
[`resolve-lint.ts`](../packages/cli/src/resolve-lint.ts) resolves the bundled
launcher and supplies it to Oxlint through the existing internal
`OXLINT_TSGOLINT_PATH` interface. Both `vp lint` and `vp check` use this resolver;
the global CLI is not required. Users keep the existing commands and
`lint.options.typeAware` / `lint.options.typeCheck` configuration.

Version output, `vite-plus/versions`, and toolchain metadata identify `tsrs`.
The global version command remains compatible with older Go-backed installations.
Ordinary project tasks that invoke `tsc` keep their selected compiler.

## Dependency changes

Compare the package declarations on `main` at `9f44ce760` with this draft:

| Dependency                | Before                                      | After                                                                            |
| ------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------- |
| Linter frontend           | `oxlint@1.93.0`                             | Unchanged                                                                        |
| Production backend        | `oxlint-tsgolint@7.0.2006`                  | `tsrs@0.24.0-ts7.1.0-beta`                                                       |
| Development-only backend  | None                                        | `oxlint-tsgolint@7.0.2006` for comparisons                                       |
| Backend platform packages | `@oxlint-tsgolint/*`: six optional packages | `@tsrs/*`: seven optional packages                                               |
| Backend package contents  | Launcher and Go executable                  | Launchers, Rust executable, TypeScript library declarations, and JavaScript APIs |

Both backends use platform packages for their native executables. tsrs has
separate Linux glibc and musl packages, but no Windows ARM64 package. A normal
production installation selects the matching platform package. Repository
development installs retain both backends for comparisons.

Oxlint declares `oxlint-tsgolint` as an optional peer dependency; it does not
require the Go backend in production. The existing project `typescript`
dependency is unchanged. tsrs supplies its own compiler and library declarations
for type-aware linting and optional type checking.

The [existing disk comparison](../bench/tsrs-lint.md#installed-disk-footprint)
measured these Linux x64/glibc production installations:

| On-disk scope           | Vite+ `1.5.0`, `oxlint-tsgolint@7.0.2005` | Preview, `tsrs@0.22.0-ts7.1.0-beta` | Increase          |
| ----------------------- | ----------------------------------------- | ----------------------------------- | ----------------- |
| Complete `node_modules` | 132.93 MiB                                | 150.87 MiB                          | 17.94 MiB (13.5%) |
| Backend packages        | 21.63 MiB                                 | 39.69 MiB                           | 18.06 MiB (83.5%) |

The complete installation includes other release-to-preview dependency changes;
the backend row isolates the backend packages. These are historical measurements,
not sizes for the current `0.24.0` integration. Repeat them before release.

## Compatibility

The pinned release targets TypeScript `7.1.0-beta`. Changing the backend also
changes the compiler version, so identical diagnostics are not guaranteed.
The [ecosystem report](../bench/tsrs-lint-ecosystem.md) records the four known
compatibility categories and their upstream attribution. Matching the pinned
TypeScript compiler does not make every new diagnostic desirable.

With type checking enabled, `0.24.0` checks whole programs for the selected
tsconfigs before running lint rules. Unselected and ignored program files can
report type errors; lint ignore patterns still control lint rules. Files outside
discovered configs use an inferred program. This broader scope can increase the
work per invocation.

The backend respects each project's compiler options. `skipLibCheck` controls
declaration checking; it is not a requirement for using tsrs. This draft aligns
the two starter templates with the repository's existing setting and supplies
the docs theme's missing Vue type declaration. Existing projects need their own
configuration review.

## Release conditions

- Repeat elapsed-time, memory, and installed-size comparisons with the release
  candidate. Keep historical measurements labeled with their original versions.
- Validate lint rules, fixes, suppression comments, and optional type checking
  on representative ecosystem projects. Track compiler-version changes separately
  from Rust implementation defects.
- Validate all supported runtime targets. The pinned release has no Windows
  ARM64 binary, and this draft implements no automatic Go fallback. Resolve that
  platform gap before a release that supports all Vite+ targets.

If the candidate fails these conditions, retain the Go backend for the release
or revert the bundled-backend change. Backend selection remains a toolchain
decision rather than an additional user setup step.

# RFC: `vp release` Command

## Summary

`vp release` versions and publishes workspace packages from conventional commits, package metadata, and git release tags. The user experience is split into two phases:

1. local `--dry-run` planning and verification
2. real publish from trusted-publishing CI

## Motivation

Publishing a multi-package workspace usually requires stitching together several fragile steps:

```bash
# Typical ad-hoc release flow
changeset version
pnpm -r build
pnpm -r publish
git tag ...
git push ...
```

Pain points:

- no single release boundary for versioning, publish, and tags
- retries are hard after a partial publish
- internal dependency ranges can drift
- local dry-runs often fail to resemble the real publish path
- CI workflows duplicate release logic instead of reusing the CLI

`vp release` aims to make publishing feel like the rest of Vite+: one command, one predictable flow, one documented operator experience.

## Goals

- Make release planning visible before any mutation happens.
- Derive versions from conventional commits and existing tags.
- Support monorepo publishing with dependency-aware ordering.
- Keep local dry-runs safe and informative.
- Make partial publish retries explicit and recoverable.
- Default real publishes to trusted-publishing CI.

## Non-Goals

- Replacing npm dist-tag semantics with a custom channel system
- Managing post-release announcement workflows
- Replacing package-manager-native publish implementations

## User Experience

### Local Preview

Operators should be able to preview a release from a clean local checkout:

```bash
vp release --dry-run
```

The preview should show:

- packages that will be released
- current and next versions
- detected release checks
- trusted publishing readiness
- publish commands that would run
- tags that would be created

### Real Release

The real release is intended for CI:

```bash
vp release --yes
```

The CLI should:

1. validate release options and trusted-publishing posture
2. compute the release plan
3. run release checks by default
4. run native publish preflight
5. publish packages
6. persist final manifests and changelogs
7. create the release commit and tags

## Release Boundary

`vp release` uses git tags as the durable watermark for future runs.

### Package Tags

Each published package gets a tag:

```text
release/<package>/v<version>
```

Examples:

```text
release/vite-plus/v1.2.3
release/voidzero-dev/vite-plus-core/v1.2.3
```

### Repository Tag

When all selected packages land on the same version, `vp release` also creates:

```text
v<version>
```

This repository tag is used for GitHub Releases and repo-wide release notes.

## Version Sources

### Conventional Commits

Version bumps come from conventional commits:

- `feat` -> minor
- `fix`, `perf`, `refactor`, `revert` -> patch
- breaking changes -> major

For `0.y.z`, breaking changes are intentionally downgraded to the minor line.

### Tag-Sourced Packages

Some packages keep `"version": "0.0.0"` in source control and treat git tags as the real version history. `vp release` should support that workflow and still rewrite manifests correctly at publish time.

## Prereleases and Retries

### Prerelease Channels

Standard prerelease channels:

```bash
vp release --preid alpha
vp release --preid beta
vp release --preid rc
```

Custom channels are also valid:

```bash
vp release --preid canary
```

Custom prerelease tags must round-trip through repository tags so retries and follow-up releases stay consistent.

### Exact Version Retries

If a publish succeeds for only part of the package set, operators should narrow the remaining packages and rerun with `--version`:

```bash
vp release --projects vite-plus --version 1.2.3 --yes
```

The CLI should infer the effective dist-tag from the exact target version when possible, instead of assuming `latest`.

## Release Checks

The default release checks should come from familiar script names:

- `build`
- `pack`
- `prepack`
- `prepublishOnly`
- `prepare`
- `release.checkScripts` in `vite.config`

Real releases run these checks by default. Dry-runs should remain lightweight unless `--run-checks` is requested.

## First Release

The first publish needs extra operator guidance:

- required workflow permissions for trusted publishing
- matching `repository` metadata
- `publishConfig.access = "public"` for public scoped packages
- recommended `vp release --first-release --dry-run` and CI commands

The workflow template should be a starting point, not a hidden side effect of a dry-run.

## Documentation Plan

The public docs should describe how to use `vp release`, not just how it is implemented.

This RFC pairs with:

- `/guide/release` for operator-facing usage
- a short README summary line instead of embedding the full release guide in the landing page

## Open Questions

- How much changelog generation should be configurable in the first public version?
- Should the scaffolded publish workflow use floating major actions (`@v6`) or pinned SHAs?
- Should the repository tag be optional for lockstep releases, or always created?

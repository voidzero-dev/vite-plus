# Release

`vp release` versions and publishes workspace packages from conventional commits and git tags. The intended workflow is: preview locally with `--dry-run`, then run the real publish from trusted-publishing CI.

## Overview

`vp release` is built for monorepos with multiple publishable packages:

- It detects releasable changes from conventional commits.
- It computes the next version for each selected package.
- It updates internal dependency ranges before publish.
- It runs publish preflight and release checks before a real release.
- It creates package tags like `release/pkg-name/v1.2.3`.

When every released package lands on the same version, `vp release` also creates a repository tag like `v1.2.3`.

## Recommended Workflow

### 1. Preview Locally

Use a local dry-run to inspect the release plan without mutating files:

```bash
vp release --dry-run
```

This shows:

- planned package versions
- detected release checks
- trusted publishing readiness
- publish command shape
- git tags that would be created

If you want the dry-run to execute detected checks too:

```bash
vp release --dry-run --run-checks
```

### 2. Publish From CI

Real publishes are designed for trusted-publishing CI:

```bash
vp release --yes
```

Use `--yes` in CI to skip the interactive confirmation prompt.

## Common Flags

### Limit the release to specific packages

```bash
vp release --projects vite-plus,@voidzero-dev/vite-plus-core --dry-run
```

When multiple package patterns are provided, their order is used as a tie-breaker for otherwise independent packages.

### Publish a prerelease

```bash
vp release --preid alpha --yes
vp release --preid beta --yes
vp release --preid rc --yes
```

Custom prerelease channels are also supported:

```bash
vp release --preid canary --yes
```

### Retry a partial publish with an exact version

If a publish stops partway through, rerun the remaining packages with an exact version:

```bash
vp release --projects vite-plus --version 1.2.3 --yes
```

## Release Checks

`vp release` looks for likely pre-release checks from:

- `build`
- `pack`
- `prepack`
- `prepublishOnly`
- `prepare`
- `vitePlus.release.checkScripts`

Real releases run these checks by default. Dry-runs stay lightweight by default, but can opt in with `--run-checks`.

## First Release

For the first publish of a workspace or package set:

```bash
vp release --first-release --dry-run
```

The first-release guidance explains:

- the publish workflow file expected by trusted publishing
- required `repository` metadata
- `publishConfig.access = "public"` for scoped public packages
- the commands to run for dry-run and real publish

## Git Tags

`vp release` uses git tags as the durable release watermark:

- package tags: `release/<package>/v<version>`
- repository tag: `v<version>` when all selected packages share the same target version

Real releases always create git tags after a successful publish. Preview-only shortcuts such as `--skip-publish` and `--no-git-tag` are restricted to `--dry-run`.

## Configuration

Release-specific check scripts can be added in `package.json`:

```json
{
  "vitePlus": {
    "release": {
      "checkScripts": ["release:verify"]
    }
  }
}
```

Use this when your publish validation does not fit the default script names.

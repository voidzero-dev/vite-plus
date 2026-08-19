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

## Configure Trusted Publishing

Vite+ can configure the npm registry relationship for every selected workspace package. It
detects the GitHub repository and publish workflow, provisions a compatible npm CLI, and invokes
`npm trust` with an explicit package list:

```bash
# Validate without changing registry state
vp release --setup-trusted-publishing --dry-run

# Configure every public workspace package
vp release --setup-trusted-publishing --yes
```

Use `--projects` to configure only part of the workspace. The repository, workflow, optional
GitHub environment, and registry can be overridden with
`--trusted-publisher-repository`, `--trusted-publisher-workflow`,
`--trusted-publisher-environment`, and `--trusted-publisher-registry`. Add
`--allow-stage-publish` when the same relationship should also authorize npm staged publishing.
When binding an existing workflow to `--trusted-publisher-environment`, ensure its publish job uses
the exact same GitHub Actions environment name; newly scaffolded workflows add this automatically.

The package must already exist on the registry before npm can attach a trusted publisher. For a
brand-new package name, perform the one-time bootstrap publish with interactive web/passkey
authentication, then run the setup command. npm allows one trusted-publisher relationship per
package and refuses to replace it implicitly. Inspect an existing relationship with
`npm trust list <package>` and explicitly revoke it before changing providers or workflow claims.

The generated GitHub Actions workflow grants `id-token: write` and does not require an npm publish
token. Commit and push the workflow before running the first OIDC release.

### Package-manager behavior

Registry configuration always uses managed npm because `npm trust` is the only registry setup CLI.
The release itself keeps native packaging semantics and chooses the safest available OIDC path:

| Project manager                | Trusted publish path                     |
| ------------------------------ | ---------------------------------------- |
| npm 11.5.1+                    | native npm OIDC                          |
| older npm                      | compatible managed npm                   |
| pnpm 11.1.3+                   | native pnpm OIDC                         |
| older pnpm                     | pnpm pack, then managed npm OIDC publish |
| Yarn 4.10.3+ on GitHub Actions | native Yarn OIDC                         |
| Yarn 4.11+ on GitLab CI        | native Yarn OIDC                         |
| Yarn Classic                   | compatible managed npm                   |
| older modern Yarn              | Yarn pack, then managed npm OIDC publish |
| Bun                            | Bun pack, then managed npm OIDC publish  |

The pack bridges publish an immutable temporary tarball, so pnpm, Yarn, and Bun still rewrite
their workspace or catalog dependency protocols before npm performs authentication and upload.

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

Custom prerelease channels are also supported, but interactive runs ask for one extra `y/N`
confirmation so a typo does not silently create a new channel:

```bash
vp release --preid canary
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
- the one-time `--setup-trusted-publishing` command and new-package bootstrap constraint

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

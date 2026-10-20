# command_publish_yarn1

## `vp pm publish --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm publish [OPTIONS] [TARBALL|FOLDER] [-- <PASS_THROUGH_ARGS>...]

Publish package to registry

Arguments:
  [TARBALL|FOLDER]        Tarball or folder to publish
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --dry-run                  Preview without publishing
  --tag <TAG>                Publish tag
  --access <ACCESS>          Access level (public/restricted)
  --otp <OTP>                One-time password for authentication
  --no-git-checks            Skip git checks
  --publish-branch <BRANCH>  Set the branch name to publish from
  --report-summary           Save publish summary
  --provenance               Publish with provenance
  --force                    Force publish
  --json                     Output in JSON format
  -r, --recursive            Publish all workspace packages
  --filter <PATTERN>         Filter packages in monorepo
  -h, --help                 Print help

Documentation: https://viteplus.dev/guide/package-management
```

## `vp pm publish --dry-run`

should reject dry-run before Yarn Classic can publish

**Exit code:** 1

```
yarn < 4.9.3 does not support --dry-run.
```

## `vp pm publish --provenance`

should reject provenance before Yarn Classic publishes without an attestation

**Exit code:** 1

```
yarn < 4.9.0 does not support --provenance.
```

## `vp pm publish --force -- --help`

should reject force rather than silently ignore it

**Exit code:** 1

```
yarn does not support --force.
```

## `vp pm publish --recursive`

should reject workspace publishing rather than use npm

**Exit code:** 1

```
yarn does not support --recursive.
```

## `vp pm publish --filter app`

should reject workspace selection rather than publish the current package

**Exit code:** 1

```
yarn does not support --filter.
```

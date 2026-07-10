# command_pm_stage_pnpm11

## `vp pm stage --help`

should list the staged-publishing subcommands

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm stage <COMMAND>

Stage a package for publishing (npm staged publishing workflow)

Commands:
  publish   Stage a package for publishing (no 2FA required)
  list, ls  List staged versions
  view      Show details about a staged version
  download  Download the staged tarball for inspection
  approve   Promote a staged version to the live registry (2FA required)
  reject    Discard a staged version (2FA required)

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm stage publish --help`

should show stage publish options

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm stage publish [OPTIONS] [TARBALL|FOLDER] [-- <PASS_THROUGH_ARGS>...]

Stage a package for publishing (no 2FA required)

Arguments:
  [TARBALL|FOLDER]        Tarball or folder to stage
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --tag <TAG>         Publish tag
  --access <ACCESS>   Access level (public/restricted)
  --otp <OTP>         One-time password for authentication
  --dry-run           Preview without staging
  --json              Output in JSON format
  -r, --recursive     Stage all publishable workspace packages
  --filter <PATTERN>  Filter packages in monorepo
  --provenance        Stage with provenance
  --registry <URL>    Registry URL
  -h, --help          Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm stage approve --help`

should show stage approve options

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm stage approve [OPTIONS] <STAGE_ID> [-- <PASS_THROUGH_ARGS>...]

Promote a staged version to the live registry (2FA required)

Arguments:
  <STAGE_ID>              Stage ID
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --otp <OTP>       One-time password for authentication
  --registry <URL>  Registry URL
  -h, --help        Print help

Documentation: https://viteplus.dev/guide/install
```

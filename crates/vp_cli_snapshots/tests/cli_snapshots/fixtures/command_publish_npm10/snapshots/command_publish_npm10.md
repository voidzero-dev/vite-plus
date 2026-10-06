# command_publish_npm10

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

Documentation: https://viteplus.dev/guide/install
```

## `vp pm publish --dry-run -- --loglevel error`

should preview publish without actually publishing (uses npm publish --dry-run)

```
 command-publish-npm10@1.0.0
```

## `vp pm publish --dry-run --json -- --ignore-scripts --loglevel error`

should preserve JSON output without publishing

```
{
  "id": "command-publish-npm10@1.0.0",
  "name": "command-publish-npm10",
  "version": "1.0.0",
  "size": 175,
  "unpackedSize": 94,
  "shasum": "6686281e40fd0fd064b4a50c67e623b29b33743f",
  "integrity": "sha512-fsEPMhiAduZmzk6kGLc+FvHMbI4SkcvpIEE5y8TkhNTtDKR1K6nMXcmJkfHI5APVP3E1uk43H/x9trb+nx++Zw==",
  "filename": "command-publish-npm10-1.0.0.tgz",
  "files": [
    {
      "path": "package.json",
      "size": 94,
      "mode": 420
    }
  ],
  "entryCount": 1,
  "bundled": []
}
```

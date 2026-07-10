# pm_version_bun

## `vp pm version patch -- --no-git-tag-version`

Bun bumps the package version

```
<version>
```

## `vp pm version prerelease -- --preid beta --no-git-tag-version`

Bun creates a prerelease with a custom identifier

```
<version>
```

## `vp pm version 2.0.0 --json`

Bun rejects unsupported JSON output

**Exit code:** 1

```
error: Invalid argument: `--json` is not supported by Bun `version`.
* Invalid argument: `--json` is not supported by Bun `version`.
```

## `vpt print-file package.json`

verify the rejected command did not update the version

```
{
  "name": "pm-version-bun",
  "version": "1.0.2-beta.0",
  "private": true,
  "license": "MIT",
  "packageManager": "bun@1.3.14"
}
```

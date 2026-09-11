# pm_version_pnpm

## `vp pm version patch -- --no-git-tag-version`

pnpm bumps the package version

```
Version bumped successfully:
pm-version-pnpm: 1.0.0 → 1.0.1
```

## `vp pm version 2.0.0 --json -- --no-git-tag-version`

pnpm accepts an explicit version with JSON output

```
[
  {
    "name": "pm-version-pnpm",
    "currentVersion": "1.0.1",
    "newVersion": "2.0.0",
    "path": "<workspace>/pnpm"
  }
]
```

## `vp pm version prerelease -- --preid beta --no-git-tag-version`

pnpm creates a prerelease with a custom identifier

```
Version bumped successfully:
pm-version-pnpm: 2.0.0 → 2.0.1-beta.0
```

## `vpt print-file package.json`

verify the version was updated

```
{
  "name": "pm-version-pnpm",
  "version": "2.0.1-beta.0",
  "private": true,
  "license": "MIT",
  "packageManager": "pnpm@11.0.6"
}
```

# pm_version_npm

## `vp pm version patch -- --no-git-tag-version`

npm bumps the package version

```
<version>
```

## `vp pm version 2.0.0 --json -- --no-git-tag-version`

npm accepts an explicit version with JSON output

```
<version>
```

## `vp pm version prerelease -- --preid beta --no-git-tag-version`

npm creates a prerelease with a custom identifier

```
<version>
```

## `vpt print-file package.json`

verify the version was updated

```
{
  "name": "pm-version-npm",
  "version": "2.0.1-beta.0",
  "private": true,
  "license": "MIT",
  "packageManager": "npm@11.11.1"
}
```

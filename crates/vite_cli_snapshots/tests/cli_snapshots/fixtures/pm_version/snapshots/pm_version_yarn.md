# pm_version_yarn

## `vp pm version patch --json -- --no-git-tag-version`

Yarn Classic bumps the package version with JSON output

```
{"type":"info","data":"Current version: 1.0.0"}
{"type":"info","data":"New version: 1.0.1"}
```

## `vp pm version 2.0.0 --json -- --no-git-tag-version`

Yarn Classic accepts an explicit version with JSON output

```
{"type":"info","data":"Current version: 1.0.1"}
{"type":"info","data":"New version: 2.0.0"}
```

## `vp pm version prerelease --json -- --preid beta --no-git-tag-version`

Yarn Classic creates a prerelease with a custom identifier

```
{"type":"info","data":"Current version: 2.0.0"}
{"type":"info","data":"New version: 2.0.1-beta.0"}
```

## `vpt print-file package.json`

verify the version was updated

```
{
  "name": "pm-version-yarn",
  "version": "2.0.1-beta.0",
  "private": true,
  "license": "MIT",
  "packageManager": "yarn@1.22.22"
}
```

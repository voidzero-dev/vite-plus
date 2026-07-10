# pm_version_yarn

## `vp pm version patch -- --no-git-tag-version`

Yarn Classic bumps the package version

```
yarn version <version>
info Current version: 1.0.0
info New version: 1.0.1
✨  Done in <duration>.
```

## `vp pm version 2.0.0 --json -- --no-git-tag-version`

Yarn Classic accepts an explicit version with JSON output

```
{"type":"info","data":"Current version: 1.0.1"}
{"type":"info","data":"New version: 2.0.0"}
```

## `vp pm version prerelease -- --preid beta --no-git-tag-version`

Yarn Classic creates a prerelease with a custom identifier

```
yarn version <version>
info Current version: 2.0.0
info New version: 2.0.1-beta.0
✨  Done in <duration>.
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

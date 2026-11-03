# pm_patch_npm12

## `vp install --ignore-scripts`


## `vp pm patch is-number -- --edit-dir edit`

npm 12 prepares a dependency for editing

```
You can now edit the following directory: <workspace>/npm12/edit
When done, run: npm patch commit <workspace>/npm12/edit
```

## `vpt stat-file edit/package.json --assert file`

```
edit/package.json: file
```

## `vpt write-file edit/vp-patch-probe.txt 'patched by vp
'`


## `vp pm patch-commit edit -- --patches-dir .patches --keep-edit-dir`

npm 12 saves the patch and registers it in the manifest and lockfile

```
npm warn shrinkwrap patchedDependencies requires lockfileVersion 4; upgrading the lockfile from version 3.
npm warn shrinkwrap Converting lock file (package-lock.json) from v3 -> v4

changed 1 package in <duration>
Patched is-number@7.0.0 -> .patches/is-number@7.0.0.patch
```

## `vpt stat-file .patches/is-number@7.0.0.patch --assert file`

```
.patches/is-number@7.0.0.patch: file
```

## `vpt stat-file edit --assert dir`

```
edit: dir
```

## `vpt print-file package.json`

```
{
  "name": "pm-patch-npm12",
  "version": "1.0.0",
  "private": true,
  "packageManager": "npm@12.0.2",
  "dependencies": {
    "is-number": "7.0.0"
  },
  "patchedDependencies": {
    "is-number@7.0.0": ".patches/is-number@7.0.0.patch"
  }
}
```

## `vpt rm -r node_modules`


## `vp install --frozen-lockfile --ignore-scripts`

a clean install reapplies the committed patch

```
VITE+ - The Unified Toolchain for the Web

added 1 package in <duration>
```

## `vpt print-file node_modules/is-number/vp-patch-probe.txt`

```
patched by vp
```

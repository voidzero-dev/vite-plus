# command_install_auto_pins_lockfile_manager

A lockfile-inferred manager must become an exact project pin so later installs cannot drift to a newer release.

## `vpt write-file package.json '{"name":"lockfile-project","private":true}
'`


## `vpt touch-file pnpm-lock.yaml`


## `vp install --silent`

installing with a lockfile-inferred manager records the exact resolved version

```
```

## `vpt print-file package.json`

the inferred manager is pinned in devEngines

```
{
  "name": "lockfile-project",
  "private": true,
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

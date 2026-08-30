# command_install_auto_pins_lockfile_manager

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

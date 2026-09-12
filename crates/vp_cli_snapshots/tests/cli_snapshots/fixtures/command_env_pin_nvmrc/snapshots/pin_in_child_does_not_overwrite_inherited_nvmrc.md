# pin_in_child_does_not_overwrite_inherited_nvmrc

## `vpt mkdir child`


## `vpt write-file child/package.json '{"name":"child"}
'`


## `cd child && vp env pin`

```
VITE+ - The Unified Toolchain for the Web

No version pinned in current directory.
  Inherited: 20.18.0 from <workspace>/.nvmrc

No package manager pinned.
```

## `cd child && vp env pin 22.13.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated devEngines.runtime in <workspace>/child/package.json
note: Version will be downloaded on first use.
```

## `vpt print-file .nvmrc package.json child/package.json`

```
# Node for local tools and CI
<version> # keep this comment
{
  "name": "env-pin-nvmrc",
  "private": true
}
{
  "name": "child",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt stat-file child/.nvmrc --assert missing`

```
child/.nvmrc: missing
```

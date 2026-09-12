# dev_engines_keeps_priority_over_nvmrc

## `vpt write-file package.json '{"devEngines":{"runtime":{"name":"node","version":"20.18.0","onFail":"warn"}}}
'`


## `vp env pin 22.13.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated devEngines.runtime in <workspace>/package.json
note: Version will be downloaded on first use.
```

## `vp env pin`

```
VITE+ - The Unified Toolchain for the Web

Pinned version: 22.13.0
  Source: <workspace>/package.json (devEngines.runtime)

No package manager pinned.
```

## `vpt print-file .nvmrc package.json`

```
# Node for local tools and CI
<version> # keep this comment
{
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "warn"
    }
  }
}
```

## `vp env unpin node`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed devEngines.runtime node entry from <workspace>/package.json
```

## `vpt stat-file .nvmrc --assert file`

```
.nvmrc: file
```

## `vpt print-file package.json`

```
{}
```

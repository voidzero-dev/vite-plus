# engines_node_does_not_make_pin_write_a_shadowed_nvmrc

## `vpt write-file package.json '{"engines":{"node":">=20.18.0"}}
'`


## `vp env pin 22.13.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated devEngines.runtime in <workspace>/package.json
note: Version will be downloaded on first use.
```

## `vpt print-file .nvmrc package.json`

```
# Node for local tools and CI
<version> # keep this comment
{
  "engines": {
    "node": ">=20.18.0"
  },
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp env current node --json`

```
{
  "node": {
    "version": "22.13.0",
    "source": "devEngines.runtime",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node",
    "installed": false,
    "mode": "managed"
  }
}
```

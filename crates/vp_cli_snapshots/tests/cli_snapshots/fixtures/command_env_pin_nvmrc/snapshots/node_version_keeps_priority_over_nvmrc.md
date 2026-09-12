# node_version_keeps_priority_over_nvmrc

## `vpt write-file .node-version '20.18.0
'`


## `vp env pin 22.13.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Created .node-version in <workspace>
note: Version will be downloaded on first use.
```

## `vpt print-file .node-version .nvmrc package.json`

```
22.13.0
# Node for local tools and CI
<version> # keep this comment
{
  "name": "env-pin-nvmrc",
  "private": true
}
```

## `vp env unpin node`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed .node-version from <workspace>
```

## `vpt stat-file .node-version --assert missing`

```
.node-version: missing
```

## `vpt stat-file .nvmrc --assert file`

```
.nvmrc: file
```

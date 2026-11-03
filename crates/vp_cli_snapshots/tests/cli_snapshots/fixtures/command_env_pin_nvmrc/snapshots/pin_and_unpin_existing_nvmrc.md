# pin_and_unpin_existing_nvmrc

## `vp env pin`

```
VITE+ - The Unified Toolchain for the Web

Pinned version: 20.18.0
  Source: <workspace>/.nvmrc

No package manager pinned.
```

## `vp env pin 22.13.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated .nvmrc in <workspace>
note: Version will be downloaded on first use.
```

## `vpt print-file .nvmrc package.json`

update the existing version token and preserve the manifest and comment

```
# Node for local tools and CI
22.13.0 # keep this comment
{
  "name": "env-pin-nvmrc",
  "private": true
}
```

## `vpt stat-file .node-version --assert missing`

```
.node-version: missing
```

## `vp env pin`

```
VITE+ - The Unified Toolchain for the Web

Pinned version: 22.13.0
  Source: <workspace>/.nvmrc

No package manager pinned.
```

## `vp env current node --json`

```
{
  "node": {
    "version": "22.13.0",
    "source": ".nvmrc",
    "source_path": "<workspace>/.nvmrc",
    "project_root": "<workspace>",
    "bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node",
    "installed": false,
    "mode": "managed"
  }
}
```

## `vp env unpin node`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed .nvmrc from <workspace>
```

## `vpt stat-file .nvmrc --assert missing`

```
.nvmrc: missing
```

## `vpt print-file package.json`

```
{
  "name": "env-pin-nvmrc",
  "private": true
}
```

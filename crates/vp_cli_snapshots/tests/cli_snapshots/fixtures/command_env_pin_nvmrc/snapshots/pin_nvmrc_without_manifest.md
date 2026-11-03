# pin_nvmrc_without_manifest

## `vpt rm package.json`


## `vp env pin 22.13.0 --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated .nvmrc in <workspace>
note: Version will be downloaded on first use.
```

## `vpt print-file .nvmrc`

```
# Node for local tools and CI
22.13.0 # keep this comment
```

## `vpt stat-file .node-version --assert missing`

```
.node-version: missing
```

## `vpt stat-file package.json --assert missing`

```
package.json: missing
```

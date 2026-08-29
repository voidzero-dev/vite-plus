# shim_pnpm_uses_project_node_version

## `vp install -g pnpm@11.20.0`

Install a fixed JS-based pnpm version


## `vp env exec node -v`

Node version resolved from .node-version

```
<version>
```

## `vp env exec pnpm --silent exec node -v`

pnpm should use same project Node version

```
<version>
```

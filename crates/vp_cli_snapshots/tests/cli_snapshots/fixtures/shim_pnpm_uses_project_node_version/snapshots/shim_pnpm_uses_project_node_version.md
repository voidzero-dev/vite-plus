# shim_pnpm_uses_project_node_version

## `vp install -g pnpm`

Ensure pnpm is globally installed


## `vp env exec node -v`

Node version resolved from .node-version

```
<version>
```

## `vp env exec pnpm exec node -v`

pnpm should use same project Node version

```
<version>
```

# shim_inherits_parent_engines_node

## `vp env exec node -v`

Root: uses engines.node directly

```
<version>
```

## `cd packages/app && vp env exec node -v`

Sub-package: inherits parent engines.node

```
<version>
```

## `vpt stat-file packages/app/.node-version --assert-not file`

Verify no .node-version created in sub-package

```
packages/app/.node-version: missing
```

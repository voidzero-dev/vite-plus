# shim_inherits_parent_node_version

## `vp env exec node -v`

Root: uses .node-version directly

```
<version>
```

## `cd packages/app && vp env exec node -v`

Sub-package: inherits parent .node-version

```
<version>
```

## `vpt stat-file packages/app/.node-version --assert-not file`

Verify no .node-version created in sub-package

```
packages/app/.node-version: missing
```

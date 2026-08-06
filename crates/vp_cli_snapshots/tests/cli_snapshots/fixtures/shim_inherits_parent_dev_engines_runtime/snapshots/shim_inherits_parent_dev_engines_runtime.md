# shim_inherits_parent_dev_engines_runtime

## `vp env exec node -v`

Root: uses devEngines.runtime directly

```
<version>
```

## `cd packages/app && vp env exec node -v`

Sub-package: inherits parent devEngines.runtime

```
<version>
```

## `vpt stat-file packages/app/.node-version --assert-not file`

Verify no .node-version created in sub-package

```
packages/app/.node-version: missing
```

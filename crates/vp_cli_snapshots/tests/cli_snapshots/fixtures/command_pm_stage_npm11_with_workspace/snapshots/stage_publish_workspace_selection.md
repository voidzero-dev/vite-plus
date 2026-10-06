# stage_publish_workspace_selection

## `vp pm stage publish --recursive --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

Recursive selects both workspaces without publishing.

```
 vp-stage-workspace-a@1.0.0 (staged)
 vp-stage-workspace-b@1.0.0 (staged)
```

## `vp pm stage publish --filter vp-stage-workspace-a --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

A single filter selects only workspace a.

```
 vp-stage-workspace-a@1.0.0 (staged)
```

## `vp pm stage publish --filter vp-stage-workspace-a --filter vp-stage-workspace-b --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

Repeated filters select both workspaces.

```
 vp-stage-workspace-a@1.0.0 (staged)
 vp-stage-workspace-b@1.0.0 (staged)
```

## `vp pm stage publish . --recursive --filter vp-stage-workspace-b --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

An explicit dot target preserves selection, and the filter narrows recursive to b.

```
 vp-stage-workspace-b@1.0.0 (staged)
```

## `vp pm stage publish ./packages/a --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

An explicit folder without workspace options remains supported.

```
 vp-stage-workspace-a@1.0.0 (staged)
```

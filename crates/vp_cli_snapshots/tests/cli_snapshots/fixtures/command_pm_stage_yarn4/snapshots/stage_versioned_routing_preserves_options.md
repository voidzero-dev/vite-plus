# stage_versioned_routing_preserves_options

## `vp install -- --mode=skip-build`


## `YARN_NPM_PUBLISH_REGISTRY=http://127.0.0.1:9 vp pm stage publish --dry-run --json`

Yarn 4.16.0 retains native staged publishing without contacting the registry.

```
{"file":"package.json"}
{"name":"vp-stage-yarn-root","version":"1.0.0","registry":"http://127.0.0.1:<port>","tag":"latest","files":["package.json"],"access":null,"dryRun":true,"staged":true,"published":false,"message":"Package archive not staged (dry run)","provenance":false}
```

## `vpt stat-file .stage-prepack-ran --assert file`

```
.stage-prepack-ran: file
```

## `PATH=${workspace}/node_modules/.bin${PATH_SEPARATOR}${PATH} vp pm stage publish . --recursive --filter vp-stage-yarn-a --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

An explicit target falls back to npm and preserves workspace selection.

```
warn: yarn cannot stage a prebuilt tarball or folder; using npm stage publish for the given target
 vp-stage-yarn-a@1.0.0 (staged)
```

## `vpt json-edit package.json packageManager yarn@4.15.0`


## `PATH=${workspace}/node_modules/.bin${PATH_SEPARATOR}${PATH} vp pm stage publish --recursive --filter vp-stage-yarn-a --dry-run --registry http://127.0.0.1:9 -- --ignore-scripts --loglevel error --fetch-retries 0`

Yarn 4.15.0 falls back to npm instead of invoking its unsupported staged command.

```
warn: yarn < 4.16.0 does not support staged publishing, falling back to npm stage
 vp-stage-yarn-a@1.0.0 (staged)
```

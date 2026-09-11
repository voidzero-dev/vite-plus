# env_use_keeps_independent_session_files

## `vp env use pnpm@10.20.0`


## `vp env use yarn@1.22.22`


## `node check.cjs pnpm 10.20.0`

```
pnpm uses the expected version
```

## `node check.cjs yarn 1.22.22`

```
yarn uses the expected version
```

## `node check.cjs vp 10.19.0`

```
vp uses the expected version
```

## `vp env current pnpm --json`

current identifies the per-family session source

```
{
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": ".session-pnpm-version",
    "source_path": "<home>/.vite-plus/.session-pnpm-version",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```

## `VP_PNPM_VERSION=10.21.0 node check.cjs pnpm 10.21.0`

An environment version takes priority over the session file

```
pnpm uses the expected version
```

## `vp env use --unset pnpm`


## `node check.cjs pnpm 10.18.0`

```
pnpm uses the expected version
```

## `node check.cjs yarn 1.22.22`

```
yarn uses the expected version
```

## `vp env use --unset pm`


## `vpt stat-file $VP_HOME/.session-yarn-version --assert missing`

```
<home>/.vite-plus/.session-yarn-version: missing
```

## `node check.cjs vp 10.19.0`

```
vp uses the expected version
```

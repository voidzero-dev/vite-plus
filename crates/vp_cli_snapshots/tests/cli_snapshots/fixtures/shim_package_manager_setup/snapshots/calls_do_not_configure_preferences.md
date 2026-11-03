# calls_do_not_configure_preferences

## `node verify.mjs prepare-dispatch`


## `cd dispatch && VP_HOME=${workspace}/home PATH=${workspace}/home/bin${PATH_SEPARATOR}${workspace}/system${PATH_SEPARATOR}${PATH} ./home/bin/pnpm --version`

An interactive call with missing preferences uses the managed default without prompting.

```
managed-pnpm
```

## `cd dispatch && VP_HOME=${workspace}/home PATH=${workspace}/home/bin${PATH_SEPARATOR}${workspace}/system${PATH_SEPARATOR}${PATH} ./home/bin/pnpm --version`

A piped call uses the same default.

```
managed-pnpm
```

## `vpt stat-file dispatch/home/config.json --assert missing`

```
dispatch/home/config.json: missing
```

## `vpt stat-file dispatch/home/bin/pnpm --assert symlink`

```
dispatch/home/bin/pnpm: symlink
```

## `vpt stat-file dispatch/home/fallback-bin/pnpm --assert missing`

```
dispatch/home/fallback-bin/pnpm: missing
```

# command_self_setup_external_bundled

## `vpt mkdir -p external/bin system/bin home`


## `vpt cp $VP_HOME/bin/vp external/bin/vp`


## `vpt cp $VP_HOME/js_runtime/node/22.18.0/bin/node system/bin/node`


## `vpt chmod +x system/bin/node`


## `vpt write-file external/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.3.2"}'`


## `vpt write-file external/node_modules/vite-plus/dist/bin.js 'console.log('\''external bundled CLI: '\'' + process.argv.slice(2).join('\'' '\''));'`


## `vpt chmod 555 external/bin/vp`


## `vpt chmod 555 external/bin`


## `vpt chmod 555 external`


## `VP_HOME=${workspace}/home VP_NODE_MANAGER=no VP_PM_MANAGER=no ./external/bin/vp --help`

A read-only package prefix needs neither a marker nor registry access


## `vpt stat-file external/bin/.vp-setup-complete --assert missing`

```
external/bin/.vp-setup-complete: missing
```

## `vpt stat-file home/current --assert missing`

```
home/current: missing
```

## `vpt stat-file home/self-setup --assert dir`

```
home/self-setup: dir
```

## `VP_HOME=${workspace}/home PATH=${workspace}/system/bin${PATH_SEPARATOR}${PATH} ./external/bin/vp sync-versions --json`

A later command uses the package manager's bundled JavaScript without another setup prompt

```
external bundled CLI: sync-versions --json
```

## `VP_HOME=${workspace}/home ./external/bin/vp env off`


## `VP_HOME=${workspace}/home VP_NODE_MANAGER=yes ./external/bin/vp --help`

Later invocations preserve the user's saved management choices


## `vpt print-file home/config.json`

```
{
  "nodeShimMode": "system_first",
  "packageManagerShimModes": {
    "bun": "system_first",
    "npm": "system_first",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

## `VP_HOME=${workspace}/home ./home/bin/vp --help`

The user shim also recognizes the external installation


## `vpt stat-file home/bin/vp --assert symlink`

```
home/bin/vp: symlink
```

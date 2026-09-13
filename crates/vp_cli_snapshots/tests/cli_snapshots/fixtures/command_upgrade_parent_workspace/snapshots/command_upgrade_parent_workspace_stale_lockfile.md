# command_upgrade_parent_workspace_stale_lockfile

Regression test for #2639: an upgrade invoked outside a parent pnpm workspace must not read its stale lockfile.

## `node setup.mjs`


## `VP_HOME=${workspace}/home/.vite-plus vp upgrade 0.3.1 --force`


## `vpt stat-file home/.vite-plus/current/node_modules/vite-plus/package.json --assert file`

```
home/.vite-plus/current/node_modules/vite-plus/package.json: file
```

## `vpt stat-file home/node_modules home/apps-ts/kami/node_modules --assert missing`

```
home/node_modules: missing
home/apps-ts/kami/node_modules: missing
```

## `vpt print-file home/pnpm-lock.yaml`

The parent workspace lockfile is unchanged.

```
lockfileVersion: '9.0'

settings:
  autoInstallPeers: true
  excludeLinksFromLockfile: false

importers:
  .:
    dependencies:
      kami:
        specifier: workspace:*
        version: link:apps-ts/kami
  apps-ts/kami: {}
```

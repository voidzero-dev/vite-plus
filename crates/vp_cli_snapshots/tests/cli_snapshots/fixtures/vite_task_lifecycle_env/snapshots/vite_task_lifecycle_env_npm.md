# vite_task_lifecycle_env_npm

Regression test for #2317 using Vite+'s real managed npm distribution. `vp run`
stamps npm's CLI entry and user agent for package-manager detection.

## `vpt json-edit package.json packageManager npm@10.9.8`


## `npm --version`

Execute the pinned npm binary

```
<version>
```

## `vp run check-env`

```
$ node check-env.js ⊘ cache disabled
npm_execpath=<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm-cli.js
npm_config_user_agent=npm/<version> node/<version> <platform> <arch> workspaces/false
```

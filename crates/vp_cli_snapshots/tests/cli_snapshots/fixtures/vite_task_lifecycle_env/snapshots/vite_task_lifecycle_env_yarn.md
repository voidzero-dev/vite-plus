# vite_task_lifecycle_env_yarn

Regression test for #2317 using Vite+'s real managed Yarn distribution. `vp run`
stamps Yarn's CLI entry and user agent for package-manager detection.

## `vpt json-edit package.json packageManager yarn@4.17.1`


## `yarn --version`

Execute the pinned Yarn binary

```
4.17.1
```

## `vp run check-env`

```
$ node check-env.js ⊘ cache disabled
npm_execpath=<home>/.vite-plus/package_manager/yarn/<version>/yarn/bin/yarn.js
npm_config_user_agent=yarn/<version> npm/? node/<version> <platform> <arch>
```

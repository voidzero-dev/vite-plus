# vite_task_lifecycle_env_pnpm11

Regression test for #2317 using Vite+'s real managed pnpm 11 distribution.
`vp run` stamps pnpm's JavaScript CLI entry and Node-backed user agent.

## `vpt json-edit package.json packageManager pnpm@11.20.0`


## `pnpm --version`

Execute the pinned pnpm 11 binary

```
11.20.0
```

## `vp run check-env`

```
$ node check-env.js ⊘ cache disabled
npm_execpath=<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm.cjs
npm_config_user_agent=pnpm/<version> npm/? node/<version> <platform> <arch>
```

# vite_task_lifecycle_env_pnpm12

Regression test for #2317 using Vite+'s real managed native pnpm 12
distribution. `npm_execpath` uses the native entry and the user agent reports
`node/?` instead of leaking the Node.js version that hosts Vite+.

## `vpt json-edit package.json packageManager pnpm@12.0.0`


## `pnpm --version`

Execute the pinned native pnpm 12 binary

```
12.0.0
```

## `vp run check-env`

```
$ node check-env.js ⊘ cache disabled
npm_execpath=<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm.native
npm_config_user_agent=pnpm/<version> npm/? node/? <platform> <arch>
```

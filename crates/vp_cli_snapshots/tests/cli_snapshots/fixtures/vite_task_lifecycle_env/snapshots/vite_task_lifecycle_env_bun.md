# vite_task_lifecycle_env_bun

Boundary coverage with Vite+'s real managed Bun distribution. The narrow #2317
fix does not emulate Bun's lifecycle environment, so both values stay undefined.

## `vpt json-edit package.json packageManager bun@1.3.14`


## `bun --version`

Execute the pinned Bun binary

```
1.3.14
```

## `vp run check-env`

```
$ node check-env.js ⊘ cache disabled
npm_execpath=(undefined)
npm_config_user_agent=(undefined)
```

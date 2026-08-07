# change_passthrough_env_config

## `MY_ENV=1 vp run hello`

```
$ node -p process.env.MY_ENV
1
```

## `MY_ENV=2 vp run hello`

MY_ENV is pass-through. should hit the cache created in step 1

```
$ node -p process.env.MY_ENV ◉ cache hit, replaying
1

---
vp run: cache hit, <duration> saved.
```

## `VITE_TASK_PASS_THROUGH_ENVS=MY_ENV,MY_ENV2 MY_ENV=2 vp run hello`

cache should be invalidated because untrackedEnv config changed

```
$ node -p process.env.MY_ENV ○ cache miss: untracked env config changed, executing
2
```

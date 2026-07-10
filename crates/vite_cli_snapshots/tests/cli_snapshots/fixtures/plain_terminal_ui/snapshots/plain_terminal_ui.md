# plain_terminal_ui

## `FOO=1 vp run hello`

```
$ node hello.mjs
input_content 1
```

## `FOO=1 vp run hello`

hit cache

```
$ node hello.mjs ◉ cache hit, replaying
input_content 1

---
vp run: cache hit, <duration> saved.
```

## `FOO=2 vp run hello`

env changed

```
$ node hello.mjs ○ cache miss: env 'FOO' changed, executing
input_content 2
```

## `FOO=2 BAR=1 vp run hello`

env added

```
$ node hello.mjs ○ cache miss: env 'BAR' changed, executing
input_content 2
```

## `vp run hello`

env removed

```
$ node hello.mjs ○ cache miss: envs 'BAR', 'FOO' changed, executing
input_content undefined
```

## `vpt write-file input.txt 'bar
'`

```
```

## `vp run hello`

input changed

```
$ node hello.mjs ○ cache miss: 'input.txt' modified, executing
bar undefined
```

## `VITE_TASK_PASS_THROUGH_ENVS=PTE vp run hello`

untrackedEnv changed

```
$ node hello.mjs ○ cache miss: untracked env config changed, executing
bar undefined
```

## `VITE_TASK_PASS_THROUGH_ENVS=PTE VITE_TASK_CWD=subfolder vp run hello`

cwd changed

```
~/subfolder$ node hello.mjs ○ cache miss: working directory changed, executing
hello from subfolder
```

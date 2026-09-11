# plain_terminal_ui_nested

## `vp run hello`

```
$ vp lint ./src
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.

$ vp lint
Found 0 warnings and 0 errors.
Finished in <duration> on 3 files with <n> rules using <n> threads.

---
vp run: 0/2 cache hit (0%). (Run `vp run --last-details` for full details)
```

## `vpt write-file a.ts 'console.log(123)
'`

```
```

## `vp run hello`

report cache status from the inner runner

```
$ vp lint ./src ◉ cache hit, replaying
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.

$ vp lint ○ cache miss: 'a.ts' modified, executing
Found 0 warnings and 0 errors.
Finished in <duration> on 3 files with <n> rules using <n> threads.

---
vp run: 1/2 cache hit (50%), <duration> saved. (Run `vp run --last-details` for full details)
```

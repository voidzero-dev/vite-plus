# command_staged_concurrent

## `git init`


## `git add -A`


## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m init`


## `vpt write-file a.txt 'changed
'`


## `git add a.txt`


## `TMPDIR=${workspace} vp staged --no-concurrent --verbose`

--no-concurrent should execute staged tasks instead of stalling

```
✔ Backed up original state in git stash (<hash>)
✔ Running tasks for staged files...
✔ Applying modifications from tasks...
✔ Cleaning up temporary files...

→ vpt print linted:
linted <workspace>/a.txt
```

## `vp staged --concurrent=0`

zero concurrency should fail before starting lint-staged

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Option "--concurrent" must be true, false, or a number greater than 0.
```

## `vp staged --no-cwd`

negated string options should report a CLI error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Option "--no-cwd" is not supported. Use "--cwd <path>".
```

## `vp staged --no-diff`

negated diff should report a CLI error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Option "--no-diff" is not supported. Use "--diff <string>".
```

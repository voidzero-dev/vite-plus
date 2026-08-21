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

error: invalid value '0' for '--concurrent [<number|boolean>]': must be true, false, or an integer from 1 to 4294967295

For more information, try '--help'.
```

## `vp staged --no-cwd`

negated string options should report a CLI error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: unexpected argument '--no-cwd' found

Usage: vp staged [OPTIONS]

For more information, try '--help'.
```

## `vp staged --no-diff`

negated diff should report a CLI error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: unexpected argument '--no-diff' found

Usage: vp staged [OPTIONS]

For more information, try '--help'.
```

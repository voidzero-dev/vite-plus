# command_staged_no_stash_restore

A failed task with --no-stash must restore the unstaged part of a partially staged file.

## `git init`


## `git add -A`


## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m init`


## `vpt write-file file.txt 'staged
'`


## `git add file.txt`


## `vpt write-file file.txt 'staged
unstaged
'`


## `vp staged --no-stash --quiet`

**Exit code:** 1

```
✖ node fail.cjs
```

## `vpt print-file file.txt`

The working tree retains both staged and unstaged changes.

```
staged
unstaged
```

## `git show :file.txt`

The index retains only the staged change.

```
staged
```

## `git stash list`

No backup stash is created.

```
```

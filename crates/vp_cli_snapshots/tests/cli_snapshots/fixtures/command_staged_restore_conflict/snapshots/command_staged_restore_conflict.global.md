# command_staged_restore_conflict

A restore conflict in a linked worktree reports successful rollback without naming a deleted patch.

## `git init`


## `git add -A`


## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m init`


## `git worktree add --detach linked HEAD`


## `vpt write-file linked/file.txt 'a
staged
c
'`


## `git -C linked add file.txt`


## `vpt write-file linked/file.txt 'a
unstaged
c
'`


## `vp staged --cwd linked`

**Exit code:** 1

```
✔ node reformat.cjs
✖ Failed to restore unstaged changes!
✔ Done reverting to original state!
Original state was restored from the backup stash.
```

## `vpt print-file linked/file.txt`

The working tree retains the unstaged change.

```
a
unstaged
c
```

## `git -C linked show :file.txt`

The index retains the staged change.

```
a
staged
c
```

## `vpt list-dir .git/worktrees/linked/lint-staged`

The temporary recovery directory is empty.

```
```

## `git stash list`

The backup stash was cleaned up after successful rollback.

```
```

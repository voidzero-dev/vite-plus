# command_staged_output

Seven staged files produce one command result; quiet, verbose, debug, and failures retain their behavior.

## `git init`


## `git add package.json vite.config.ts fail.mjs output.mjs`


## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m init`


## `git add file1.ts file2.ts file3.ts file4.ts file5.ts file6.ts file7.ts`


## `vp staged`

One result for seven files in a terminal.

```
✔ vp check --fix
```

## `git diff --exit-code`

Task fixes are staged.

```
```

## `git show :file1.ts`

The staged content was formatted.

```
export const value1 = 1;
```

## `git stash list`

The backup stash was cleaned up.

```
```

## `vp staged`

Piped output is compact too.

```
✔ vp check --fix
```

## `vp staged --quiet`

Quiet mode has no success output.

```
```

## `vp staged --verbose`

Verbose mode retains successful task output.

```
✔ vp check --fix

→ vp check --fix:
pass: Formatting completed for checked files (<duration>)
pass: Found no warnings or lint errors in 7 files (<duration>, <n> threads)
```

## `vp staged --debug`

Debug mode still runs, but its full workflow log is not recorded.


## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m checked`


## `vp staged`

No staged files is still reported.

```
→ lint-staged could not find any staged files.
```

## `vpt write-file note.txt 'no matching task
'`


## `git add note.txt`


## `vp staged`

No matching tasks is still reported.

```
→ lint-staged could not find any staged files matching configured tasks.
```

## `vpt write-file output.out 'output
'`


## `git add output.out`


## `vp staged --quiet --verbose`

Quiet verbose mode preserves indented task output.

```
    Task output
```

## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m output`


## `vpt write-file fail.js 'export const fail = true;
'`


## `git add fail.js`


## `vp staged`

Failures retain diagnostics and rollback status.

**Exit code:** 1

```
✖ node fail.mjs
✖ Failed to run tasks for staged files!
↓ Skipped staging changes from tasks…
✔ Done reverting to original state!

✖ node fail.mjs:
Task diagnostic
```

## `git diff --exit-code`

The working tree was restored.

```
```

## `git show :fail.js`

The index retains the original staged content.

```
export const fail = true;
```

## `git stash list`

Failure cleanup removed the backup stash.

```
```

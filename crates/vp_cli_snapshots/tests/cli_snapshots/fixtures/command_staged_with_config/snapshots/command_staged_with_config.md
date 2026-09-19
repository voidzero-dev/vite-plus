# command_staged_with_config

## `git init`


## `git add -A`


## `git commit -m init`


## `vpt write-file src/index.ts 'export const hello = '\''world'\'';
export const foo = 1;
'`

append foo (write-file with the full appended content)


## `git add src/index.ts`


## `vp staged`

should succeed with staged .ts files

```
⋯ Backing up original state…
✔ Done backing up original state (<hash>)!
⋯ Running tasks for staged files…
    *.ts — 1 file
      ⋯ vp check --fix

✔ vp check --fix

✔ Done running tasks for staged files!
⋯ Staging changes from tasks…
✔ Done staging changes from tasks!
⋯ Cleaning up temporary files…
✔ Done cleaning up temporary files!
```

## `git add -A`


## `git commit -m second`


## `vpt write-file src/index.ts 'export const hello = '\''world'\'';
export const foo = 1;
export const bar = 2;
'`

append bar


## `git add src/index.ts`


## `vp staged --debug`

should succeed with debug enabled


## `git add -A`


## `git commit -m third`


## `vpt write-file src/fail.js 'eval("code");
'`


## `git add src/fail.js`


## `vp staged`

should fail when staged .js file has lint errors

**Exit code:** 1

```
⋯ Backing up original state…
✔ Done backing up original state (<hash>)!
⋯ Running tasks for staged files…
    *.js — 1 file
      ⋯ vp lint

✖ vp lint

✖ Failed to run tasks for staged files!
↓ Skipped staging changes from tasks…
⋯ Reverting to original state because of errors…
✔ Done reverting to original state!
⋯ Cleaning up temporary files…
✔ Done cleaning up temporary files!

✖ vp lint:

  × eslint(no-eval): eval can be harmful.
   ╭─[src/fail.js:1:1]
 1 │ eval("code");
   · ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

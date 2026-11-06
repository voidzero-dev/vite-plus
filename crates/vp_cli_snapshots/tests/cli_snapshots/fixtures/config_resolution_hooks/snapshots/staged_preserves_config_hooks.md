# staged_preserves_config_hooks

## `git init --quiet`


## `git add input.txt`


## `git commit --quiet -m initial`


## `vpt write-file input.txt 'changed
'`


## `git add input.txt`


## `vp staged --verbose --no-stash`

Run staged configuration supplied by a Vite plugin and print the matched files.

```
config NODE_ENV: development
apply root: <workspace>
config root: <workspace>
configResolved root: <workspace>
configResolved configFile: <workspace>/vite.config.mjs
configResolved dependencies:
  <workspace>/settings.mjs
  <workspace>/vite.config.mjs
⚠ Skipping backup because `--no-stash` was used. This might result in data loss.

✔ vpt stat-file

→ vpt stat-file:
<workspace>/input.txt: file
```

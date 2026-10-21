# vpx_script_watch

Node.js --watch restarts when an imported .ts file changes, and on a tsconfig edit, which the loader reports.

## `node verify-watch.mjs dependency --watch`

```
vpx --watch restarted after an imported .ts file changed
```

## `node verify-watch.mjs tsconfig --watch`

```
vpx --watch restarted after a tsconfig paths edit
```

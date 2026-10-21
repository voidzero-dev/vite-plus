# vpx_watch

vpx watch runs oxc-node's watcher with the script loader. It reruns the script when an imported file or a tsconfig changes, when the file a failed import looked for is created, and when the tsconfig given by --tsconfig changes.

## `node verify-watch.mjs dependency watch`

```
vpx watch restarted after an imported .ts file changed
```

## `node verify-watch.mjs tsconfig watch`

```
vpx watch restarted after a tsconfig paths edit
```

## `node verify-watch.mjs created watch`

```
vpx watch restarted after the file an import looked for was created
```

## `node verify-watch.mjs pinned --tsconfig tsconfig.b.json watch`

A vpx --tsconfig before watch is the watcher's --tsconfig

```
vpx --tsconfig tsconfig.b.json watch restarted after an edit to the tsconfig it was given
```

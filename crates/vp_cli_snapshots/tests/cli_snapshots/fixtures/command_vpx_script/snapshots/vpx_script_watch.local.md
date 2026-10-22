# vpx_script_watch

vpx --watch runs oxc-node's watcher, never Node.js' own watch mode, with the script loader. It reruns the script when an imported file or a tsconfig changes, when the file a failed import looked for is created, and when the tsconfig given by --tsconfig changes.

## `node verify-watch.mjs dependency --watch`

```
vpx --watch restarted after an imported .ts file changed
```

## `node verify-watch.mjs tsconfig --watch`

```
vpx --watch restarted after a tsconfig paths edit
```

## `node verify-watch.mjs created --watch`

```
vpx --watch restarted after the file an import looked for was created
```

## `node verify-watch.mjs pinned --tsconfig tsconfig.b.json --watch`

--tsconfig as a vpx option

```
vpx --tsconfig tsconfig.b.json --watch restarted after an edit to the tsconfig it was given
```

## `node verify-watch.mjs pinned-config --watch`

--watch before a separate config file path, and --tsconfig among the Node.js options after it, which wins over a TS_NODE_PROJECT from an env file

```
vpx --watch --experimental-config-file custom.json --tsconfig tsconfig.b.json --env-file=.env.tsconfig restarted after an edit to the tsconfig it was given
```

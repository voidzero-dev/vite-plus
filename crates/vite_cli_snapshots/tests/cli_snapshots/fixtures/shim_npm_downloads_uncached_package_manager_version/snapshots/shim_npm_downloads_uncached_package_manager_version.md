# shim_npm_downloads_uncached_package_manager_version

## `vpt write-file .node-version '22.11.0
'`

Pin the project Node.js version


## `vpt rm -rf $VP_HOME/js_runtime/node/22.11.0`

Ensure pinned Node.js is not cached


## `vpt rm -rf $VP_HOME/package_manager/npm/10.5.0 $VP_HOME/package_manager/npm/10.5.0.lock`

Ensure pinned npm is not cached


## `vpt stat-file $VP_HOME/js_runtime/node/22.11.0 --assert missing`

pinned Node.js@22.11.0 is uncached

```
<home>/.vite-plus/js_runtime/node/<version>: missing
```

## `vpt stat-file $VP_HOME/package_manager/npm/10.5.0 --assert missing`

pinned npm@10.5.0 is uncached

```
<home>/.vite-plus/package_manager/npm/<version>: missing
```

## `node check-npm-version.mjs 10.5.0 'npm shim auto-downloaded Node.js and packageManager on first invocation'`

```
npm shim auto-downloaded Node.js and packageManager on first invocation
```

## `vpt stat-file $VP_HOME/js_runtime/node/22.11.0/bin/node --assert file`

pinned Node.js@22.11.0 is now cached

```
<home>/.vite-plus/js_runtime/node/<version>/bin/node: file
```

## `vpt stat-file $VP_HOME/package_manager/npm/10.5.0/npm/bin/npm --assert file`

pinned npm@10.5.0 is now cached

```
<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm: file
```

## `node check-npm-version.mjs 10.5.0 'subsequent invocations reuse the cached version'`

```
subsequent invocations reuse the cached version
```

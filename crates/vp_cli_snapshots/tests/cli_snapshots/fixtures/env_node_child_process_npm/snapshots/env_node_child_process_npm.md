# env_node_child_process_npm

The legacy case compared `$(npm --version)` against a node child process's
`npm --version` via shell command substitution. There is no shell, but both
projects pin the resolution deterministically (node/ pins the runtime, so its
bundled npm version is fixed; specific/ pins devEngines.packageManager), so the
snapshot itself asserts the resolved versions, and print-path.js asserts the
child process resolves npm to the vp shim.

## `cd node && node ../print-version.js`

child-process npm is the pinned runtime's bundled npm

```
10.8.2
```

## `cd node && node ../print-path.js`

child-process npm resolves to the vp shim

```
<home>/.vite-plus/js_runtime/node/<version>/bin/npm
```

## `cd specific && node ../print-version.js`

child-process npm is the devEngines.packageManager pin

```
11.17.0
```

## `cd specific && node ./print-package-json-pm-version.js`

the pin itself, for comparison with the step above

```
11.17.0
```

## `cd specific && node ../print-path.js`

child-process npm resolves to the vp shim

```
<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm
```

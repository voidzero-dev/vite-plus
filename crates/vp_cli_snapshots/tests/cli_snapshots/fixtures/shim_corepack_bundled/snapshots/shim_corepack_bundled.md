# shim_corepack_bundled

## `vp remove -g corepack`

Isolate from a leftover managed corepack, which would win over the bundled one

**Exit code:** 1

```
Failed to uninstall corepack: Package corepack is not installed
```

## `vpt write-file .node-version '20.18.0
'`

Pin the project Node.js version


## `vp env exec node --version`

Ensure Node.js is installed first

```
<version>
```

## `corepack --version`

corepack shim runs the Node-bundled corepack

```
0.29.3
```

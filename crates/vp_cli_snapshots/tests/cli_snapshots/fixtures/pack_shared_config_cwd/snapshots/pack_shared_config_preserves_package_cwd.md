# pack_shared_config_preserves_package_cwd

## `vp pack --logLevel silent`

Load the parent config through traverseUp while plugin hooks and pack paths stay at the package cwd, overriding the file's root.

```
ℹ Build start
```

## `node ../../verify-pack.mjs`

```
shared config keeps plugin root and pack paths in the package
```

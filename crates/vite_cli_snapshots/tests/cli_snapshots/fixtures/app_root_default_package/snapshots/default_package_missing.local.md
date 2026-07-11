# default_package_missing

defaultPackage pointing at a missing directory errors before any workspace lookup.

## `cd missing && vp build`

**Exit code:** 1

```
error: defaultPackage points to a missing directory: ./packages/nope
```

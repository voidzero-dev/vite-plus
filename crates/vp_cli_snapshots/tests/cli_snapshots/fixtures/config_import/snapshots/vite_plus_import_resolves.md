# vite_plus_import_resolves

Locks in the fixture module contract: a config importing bare `vite-plus`
resolves through the run-root node_modules the runner provides, without
the fixture vendoring anything.

## `vp run hello`

```
$ vpt print config-loaded
config-loaded
```

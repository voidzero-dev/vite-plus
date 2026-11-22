# default_package_build_with_separator

A lone -- lets vp build use defaultPackage at the workspace root.

## `vp build --`

```
note: vp build: using ./apps/web (defaultPackage in vite.config.ts)
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vpt grep-file apps/web/dist/index.html <h1>web</h1>`

```
apps/web/dist/index.html: found "<h1>web</h1>"
```

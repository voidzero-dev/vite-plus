# vpx_package_not_installed

The project-local vpx, which package.json scripts run, searches only node_modules/.bin, never global packages or PATH. A binary that is not installed is an error with a hint, not a download.

## `vpx fake-missing-tool .`

**Exit code:** 1

```
error: vpx: 'fake-missing-tool' is not installed in node_modules
note: Add it to devDependencies, or run `vp dlx fake-missing-tool` to download and run it.
```

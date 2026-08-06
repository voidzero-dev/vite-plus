# command_pack_tsdown_extensions

## `node verify-extensions.mjs`

bundled @tsdown/exe and @tsdown/css load without a top-level tsdown (issue #1586)

```
tsdown-exe.js: getCacheDir, getCachedBinaryPath, getTargetSuffix, resolveNodeBinary
tsdown-css.js: CssPlugin, resolveCssOptions
```

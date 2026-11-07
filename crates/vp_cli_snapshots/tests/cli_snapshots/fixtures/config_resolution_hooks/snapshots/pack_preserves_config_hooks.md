# pack_preserves_config_hooks

## `vp pack --logLevel silent`

Build with plugin-supplied pack options and configuration metadata available in configResolved.

```
config NODE_ENV: development
apply root: <workspace>
config root: <workspace>
configResolved root: <workspace>
configResolved configFile: <workspace>/vite.config.mjs
configResolved dependencies:
  <workspace>/settings.mjs
  <workspace>/vite.config.mjs
ℹ Build start
```

## `vpt print-file dist/index.mjs`

```
// app/index.js
const app = "fixture";
export { app };
```

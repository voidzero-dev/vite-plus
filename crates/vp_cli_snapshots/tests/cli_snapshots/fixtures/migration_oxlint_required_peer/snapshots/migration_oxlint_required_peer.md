# migration_oxlint_required_peer

## `cd plugin && vp pm pack --pack-destination ../artifacts`


## `vpt rm -rf plugin`

Only the installed dependency contains a reference to the required peer.


## `vp install --ignore-scripts`


## `vp run check-plugin`

The installed plugin can load its required peer before migration.

```
VITE+ - The Unified Toolchain for the Web

$ node check.cjs ⊘ cache disabled
function
```

## `node -e 'const { createRequire } = require('\''node:module'\''); console.log(typeof createRequire(require.resolve('\''vite-plus/package.json'\''))('\''@oxlint/plugins'\'').defineRule);'`

Vite+ also has its own transitive copy of the plugin API.

```
function
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file package.json`

Keep the direct provider even though application source never names the peer.

```
{
  "name": "migration-oxlint-required-peer",
  "private": true,
  "scripts": {
    "check-plugin": "node check.cjs"
  },
  "devDependencies": {
    "@oxlint/plugins": "1.79.0",
    "review-oxlint-plugin": "file:artifacts/review-oxlint-plugin-1.0.0.tgz",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.24.0"
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vp run check-plugin`

The plugin still loads after reinstall with automatic peers and hoisting disabled.

```
VITE+ - The Unified Toolchain for the Web

$ node check.cjs ⊘ cache disabled
function
```

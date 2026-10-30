# migration_oxnode_shebang_sync

The local registry supplies published oxc-node 1.0.1 to compare original and migrated loader chains. The variant and image loaders are locally authored fixtures; the referenced Athena Crisis loaders are not in its public tree.

## `vpt json-edit package.json devDependencies.@oxc-node/core 1.0.1`


## `vp install --ignore-scripts`


## `vp run loader`

The original register runs TypeScript, variant resolution, image imports, and both custom loaders in order.

```
$ node --no-warnings --experimental-specifier-resolution=node --import @oxc-node/core/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js ./scripts/build-assets.tsx ⊘ cache disabled
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

## `vp run shebang`

```
$ ./scripts/build-assets.tsx ⊘ cache disabled
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

## `vp run extensionless`

```
$ ./bin/build-assets ⊘ cache disabled
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● oxnode scripts detected. Auto-migrating to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• oxnode scripts migrated to Vite+
• Package manager settings configured
```

## `vpt print-file package.json scripts/build-assets.tsx bin/build-assets`

Only the register import changes; Node.js flags, custom loader order, and script bodies stay intact.

```
{
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "name": "migration-oxnode-custom-loaders",
  "scripts": {
    "extensionless": "./bin/build-assets",
    "loader": "node --no-warnings --experimental-specifier-resolution=node --import vite-plus/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js ./scripts/build-assets.tsx",
    "shebang": "./scripts/build-assets.tsx"
  },
  "type": "module"
}
#!/usr/bin/env -S node --no-warnings --experimental-specifier-resolution=node --import vite-plus/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js
import '../src/main.ts';
#!/usr/bin/env -S node --no-warnings --experimental-specifier-resolution=node --import vite-plus/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js
import('../src/main.ts');
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/@oxc-node/core`

A fresh install no longer includes standalone oxc-node.

```
node_modules/@oxc-node/core: missing
```

## `vp run loader`

The migrated register produces the same values and loader order.

```
$ node --no-warnings --experimental-specifier-resolution=node --import vite-plus/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js ./scripts/build-assets.tsx ⊘ cache disabled
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

## `vp run shebang`

```
$ ./scripts/build-assets.tsx ⊘ cache disabled
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

## `vp run extensionless`

```
$ ./bin/build-assets ⊘ cache disabled
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

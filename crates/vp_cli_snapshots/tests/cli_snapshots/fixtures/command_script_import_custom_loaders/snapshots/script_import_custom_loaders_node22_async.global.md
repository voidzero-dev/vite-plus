# script_import_custom_loaders_node22_async

Two representative project loaders compose with the TypeScript preload. Their output also shows the hook order and require() inside loader-supplied CommonJS source.

## `node --version`

```
<version>
```

## `node --no-warnings --experimental-specifier-resolution=node --import vite-plus/register --loader ./scripts/variant-loader.js --loader ./scripts/image-loader.js ./src/main.ts`

```
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

## `node --no-warnings --import=vite-plus/register --loader=./scripts/image-loader.js --loader=./scripts/variant-loader.js ./src/main.ts`

Reversing the custom loaders reverses their observable load order.

```
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["image","variant"],"generated":{"local":"dependency","builtin":"child"}}
```

## `NODE_OPTIONS=--no-warnings --import=vite-plus/register --loader=./scripts/variant-loader.js --loader=./scripts/image-loader.js node ./src/main.ts`

The same chain works when inherited through NODE_OPTIONS.

```
{"mode":"typescript","variant":"blue","image":{"name":"logo.svg","bytes":42},"order":["variant","image"],"generated":{"local":"dependency","builtin":"child"}}
```

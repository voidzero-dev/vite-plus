# vpx_script_ts_syntax

TypeScript syntax that Node.js type stripping rejects runs through the oxc-node loader.

## `vpx ./src/syntax.ts`

Enums, namespaces, parameter properties, decorators, and a lowered class field

```
decorated sum
green 3 4 private field
```

## `vpx ./src/bom-shebang.ts`

A byte order mark before a shebang, as Windows editors save it

```
with a byte order mark before the shebang
```

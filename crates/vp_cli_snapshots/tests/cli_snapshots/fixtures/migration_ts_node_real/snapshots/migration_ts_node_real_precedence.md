# migration_ts_node_real_precedence

A deliberate TS2322 error distinguishes transpilation from type checking. Config overrides the transpile-only environment setting, while an explicit type-check option or environment setting wins over transpile-only.

## `vpt json-edit package.json scripts {}`


## `vpt json-edit package.json scripts.config-wins 'TS_NODE_TRANSPILE_ONLY=false ts-node ./scripts/type-error.ts'`


## `vpt json-edit package.json scripts.type-check 'ts-node --transpileOnly --typeCheck ./scripts/type-error.ts'`


## `vpt json-edit package.json scripts.env-type-check 'TS_NODE_TYPE_CHECK=true ts-node --transpileOnly ./scripts/type-error.ts'`


## `vp install --ignore-scripts`


## `vp run config-wins`

Before migration: transpileOnly:true in the entry config overrides TS_NODE_TRANSPILE_ONLY=false and executes the invalid assignment.

```
$ TS_NODE_TRANSPILE_ONLY=false ts-node ./scripts/type-error.ts ⊘ cache disabled
{"value":"transpile-only execution"}
```

## `vp run type-check`

Before migration: --typeCheck wins over --transpileOnly and rejects the assignment.

**Exit code:** 1

```
$ ts-node --transpileOnly --typeCheck ./scripts/type-error.ts ⊘ cache disabled
[TSError: ⨯ Unable to compile TypeScript:
scripts/type-error.ts:1:7 - error TS2322: Type 'string' is not assignable to type 'number'.

1 const value: number = 'transpile-only execution';
        ~~~~~
] {
  diagnosticCodes: [ 2322 ]
}
```

## `vp run env-type-check`

Before migration: TS_NODE_TYPE_CHECK=true also keeps type checking enabled.

**Exit code:** 1

```
$ TS_NODE_TYPE_CHECK=true ts-node --transpileOnly ./scripts/type-error.ts ⊘ cache disabled
[TSError: ⨯ Unable to compile TypeScript:
scripts/type-error.ts:1:7 - error TS2322: Type 'string' is not assignable to type 'number'.

1 const value: number = 'transpile-only execution';
        ~~~~~
] {
  diagnosticCodes: [ 2322 ]
}
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• ts-node scripts migrated to Vite+
• Package manager settings configured
! Warnings:
  - Kept ts-node in package.json scripts "env-type-check": this ts-node command type-checks before execution; `vpx` does not type-check. Keep a separate type-check command before changing this script.
  - Kept ts-node in package.json scripts "type-check": this ts-node command type-checks before execution; `vpx` does not type-check. Keep a separate type-check command before changing this script.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "@types/node": "22.18.6",
    "ts-node": "10.9.2",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node-real",
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "config-wins": "TS_NODE_TRANSPILE_ONLY=false vpx --tsconfig=scripts/tsconfig.json ./scripts/type-error.ts",
    "env-type-check": "TS_NODE_TYPE_CHECK=true ts-node --transpileOnly ./scripts/type-error.ts",
    "type-check": "ts-node --transpileOnly --typeCheck ./scripts/type-error.ts"
  }
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vp run config-wins`

After migration: vpx produces the same value for the supported transpile-only command.

```
$ TS_NODE_TRANSPILE_ONLY=false vpx --tsconfig=scripts/tsconfig.json ./scripts/type-error.ts ⊘ cache disabled
{"value":"transpile-only execution"}
```

## `vp run type-check`

After migration: the retained command still rejects the assignment.

**Exit code:** 1

```
$ ts-node --transpileOnly --typeCheck ./scripts/type-error.ts ⊘ cache disabled
[TSError: ⨯ Unable to compile TypeScript:
scripts/type-error.ts:1:7 - error TS2322: Type 'string' is not assignable to type 'number'.

1 const value: number = 'transpile-only execution';
        ~~~~~
] {
  diagnosticCodes: [ 2322 ]
}
```

## `vp run env-type-check`

After migration: the retained environment-controlled type check still rejects it.

**Exit code:** 1

```
$ TS_NODE_TYPE_CHECK=true ts-node --transpileOnly ./scripts/type-error.ts ⊘ cache disabled
[TSError: ⨯ Unable to compile TypeScript:
scripts/type-error.ts:1:7 - error TS2322: Type 'string' is not assignable to type 'number'.

1 const value: number = 'transpile-only execution';
        ~~~~~
] {
  diagnosticCodes: [ 2322 ]
}
```

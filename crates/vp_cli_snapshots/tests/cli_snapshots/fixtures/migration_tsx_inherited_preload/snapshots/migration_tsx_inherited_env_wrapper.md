# migration_tsx_inherited_env_wrapper

The env wrapper must preserve an inherited tsx preload for a CommonJS TypeScript file with imports. Real tsx from the local registry executes both commands. The wrapper and environment assignment require POSIX.

## `vpt json-edit package.json scripts.inherited 'NODE_OPTIONS=--import=tsx env node bad.cts'`


## `vp install`


## `vp run inherited`

The original env-wrapped command prints function.

```
$ NODE_OPTIONS=--import=tsx env node bad.cts ⊘ cache disabled
function
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

Full migration keeps tsx for the incompatible entry behind env.

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● tsx scripts detected. Auto-migrating to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• tsx scripts migrated to Vite+
• Package manager settings configured
! Warnings:
  - Kept tsx in package.json scripts "generate": `bad.cts` uses import or export, which `vpx` does not compile to CommonJS; rename it to .mts.
  - Kept tsx in package.json scripts "inherited": `bad.cts` uses import or export, which `vpx` does not compile to CommonJS; rename it to .mts.
  - Kept the tsx devDependency in package.json: `scripts` in package.json still uses tsx.
```

## `vpt print-file package.json`

The compatible script moves to vpx; the wrapper keeps its preload and tsx remains installed.

```
{
  "devDependencies": {
    "tsx": "4.23.15",
    "vite-plus": "catalog:"
  },
  "name": "migration-tsx-inherited-preload",
  "scripts": {
    "compatible": "vpx good.ts",
    "generate": "tsx bad.cts",
    "inherited": "NODE_OPTIONS=--import=tsx env node bad.cts"
  },
  "type": "module"
}
```

## `vp run inherited`

The migrated project still prints function.

```
$ NODE_OPTIONS=--import=tsx env node bad.cts ⊘ cache disabled
function
```

## `vp run compatible`

The compatible TypeScript script executes through vpx.

```
$ vpx good.ts ⊘ cache disabled
compatible script
```

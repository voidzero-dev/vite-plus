# migration_tsx_inherited_node_run

A Node.js script launcher passes NODE_OPTIONS to its retained tsx child. The local registry supplies published tsx so both commands execute its loader. The environment assignment uses POSIX shell syntax.

## `vp install`


## `vp run inherited`

The original inherited tsx preload executes the CommonJS TypeScript child.

```
$ NODE_OPTIONS=--import=tsx node --run generate ⊘ cache disabled
function
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

Full migration keeps the child script and its inherited tsx preload.

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
  - Kept tsx in package.json scripts "inherited": script `generate` needs tsx: `bad.cts` uses import or export, which `vpx` does not compile to CommonJS; rename it to .mts.
  - Kept the tsx devDependency in package.json: `scripts` in package.json still uses tsx.
```

## `vpt print-file package.json`

The compatible script moves to vpx; the retained child and NODE_OPTIONS keep tsx.

```
{
  "name": "migration-tsx-inherited-preload",
  "type": "module",
  "scripts": {
    "generate": "tsx bad.cts",
    "inherited": "NODE_OPTIONS=--import=tsx node --run generate",
    "compatible": "vpx good.ts"
  },
  "devDependencies": {
    "tsx": "4.23.15",
    "vite-plus": "catalog:"
  }
}
```

## `vp run inherited`

The migrated project still prints function.

```
$ NODE_OPTIONS=--import=tsx node --run generate ⊘ cache disabled
function
```

## `vp run compatible`

The compatible TypeScript script executes through vpx.

```
$ vpx good.ts ⊘ cache disabled
compatible script
```

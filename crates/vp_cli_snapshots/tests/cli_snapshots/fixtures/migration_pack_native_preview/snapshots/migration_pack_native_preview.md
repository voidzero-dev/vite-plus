# migration_pack_native_preview

Migrate a library that used native-preview for declarations and preserve the compiler of a separate workspace package.

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus   1.6.0 → <version>
    vite              → <version>
    typescript  5.9.3 → 7.1.0-beta
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file package.json`

```
{
  "name": "migration-pack-native-preview",
  "private": true,
  "type": "module",
  "types": "./dist/index.d.mts",
  "devDependencies": {
    "typescript": "7.1.0-beta",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.24.0"
}
```

## `vp install --ignore-scripts`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `node -e 'require('\''node:assert/strict'\'').equal(require('\''./package.json'\'').devDependencies['\''@typescript/native-preview'\''], undefined)'`


## `vp exec tsc --version`

```
VITE+ - The Unified Toolchain for the Web

Version 7.1.0-beta
```

## `cd packages/classic && vp exec tsc --version`

```
VITE+ - The Unified Toolchain for the Web

Version 5.9.3
```

## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
warn: TypeScript Go is experimental. Some options will be unavailable.
ℹ Emit types with TypeScript Go
ℹ dist/index.mjs    <size> kB │ gzip: <size> kB
ℹ dist/index.d.mts  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt print-file dist/index.d.mts`

```
export declare function greet(name: string): {
  message: string;
  length: number;
};
```

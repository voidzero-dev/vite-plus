# migration_standalone_npm

## `vp migrate --no-interactive --no-hooks`

migration should work with npm, add overrides, and update lockfile

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file package.json`

check package.json has overrides field (not pnpm.overrides)

```
{
  "name": "migration-standalone-npm",
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "packageManager": "npm@10.9.2"
}
```

## `vpt grep-file package-lock.json @voidzero-dev/vite-plus-core`

lockfile updated with the vite override (aliased to @voidzero-dev/vite-plus-core)

```
package-lock.json: found "@voidzero-dev/vite-plus-core"
```

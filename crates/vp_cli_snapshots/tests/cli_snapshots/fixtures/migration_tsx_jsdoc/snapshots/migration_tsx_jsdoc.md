# migration_tsx_jsdoc

A JSDoc tsx code fence must not keep the tsx dependency (#3297). The tsx package is a local stub.

## `vpt print-file use-editing.ts`

the source mentions tsx only in a JSDoc code fence

```
/**
 * @example
 * ```tsx
 * const isEditing = useEditingStore(selectIsEditing);
 * ```
 */
export function useEditingStore() {}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

migration must not warn that use-editing.ts still uses tsx

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to vpx...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to `vpx`
```

## `vpt print-file package.json`

start runs vpx, and the unused tsx devDependency is removed

```
{
  "name": "migration-tsx-jsdoc",
  "type": "module",
  "scripts": {
    "start": "vpx index.ts"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@10.33.2"
}
```

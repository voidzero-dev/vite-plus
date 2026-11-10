# migration_ignore_patterns_invalid

## `vpt write-file vite.config.ts 'export default { migrate: { ignorePatterns: '\''templates/'\'' } };
'`

```
```

## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

■ migrate.ignorePatterns must be an array of strings.
Error: migrate.ignorePatterns must be an array of strings.
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app", "private": true, "type": "module",
  "scripts": { "build": "vite build", "test": "vitest run" },
  "devDependencies": { "vite": "catalog:", "vitest": "catalog:legacy", "@vitest/coverage-v8": "catalog:legacy" }
}
```

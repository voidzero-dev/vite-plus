# migration_eslint_nuxt_skip

## `vp migrate --no-interactive`

@nuxt/eslint detected — ESLint migration is skipped with a warning

```
VITE+ - The Unified Toolchain for the Web

@nuxt/eslint detected — automatic ESLint migration is skipped. @nuxt/eslint wires ESLint into a framework-specific flow that Vite+ cannot migrate cleanly yet. Your ESLint setup is preserved. To migrate manually, remove @nuxt/eslint from package.json and re-run `vp migrate`.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file package.json`

eslint, @nuxt/eslint, and eslint.config.mjs are preserved

```
{
  "name": "migration-eslint-nuxt-skip",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "nuxt build",
    "dev": "nuxt dev",
    "lint": "eslint .",
    "prepare": "vp config"
  },
  "dependencies": {
    "nuxt": "^4.0.0"
  },
  "devDependencies": {
    "@nuxt/eslint": "^1.0.0",
    "eslint": "^9.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt stat-file eslint.config.mjs --assert file`

eslint config file is NOT deleted

```
eslint.config.mjs: file
```

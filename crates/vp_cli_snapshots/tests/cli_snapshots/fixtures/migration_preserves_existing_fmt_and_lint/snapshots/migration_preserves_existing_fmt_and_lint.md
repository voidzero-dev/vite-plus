# migration_preserves_existing_fmt_and_lint

## `vp migrate --no-interactive`

should NOT duplicate fmt/lint blocks already in vite.config.ts (regression for vp create fate)

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file vite.config.ts`

exactly one fmt: and one lint: block, preserving template values

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {
    experimentalSortImports: {
      newlinesBetween: false,
    },
    experimentalSortPackageJson: {
      sortScripts: true,
    },
    experimentalTailwindcss: {
      stylesheet: 'client/src/App.css',
    },
    ignorePatterns: [
      'coverage/',
      'dist/',
      '.fate/',
      'client/dist/',
      'client/src/translations/',
      'server/dist/',
      'pnpm-lock.yaml',
    ],
    singleQuote: true,
  },
  lint: {
    extends: ['@nkzw/oxlint-config'],
    ignorePatterns: [
      'coverage',
      'dist',
      '.fate',
      'client/dist',
      'server/dist',
      'server/src/drizzle/migrations/**',
    ],
    options: { typeAware: true, typeCheck: true },
    overrides: [
      {
        files: ['server/src/index.tsx', 'server/src/drizzle/seed.tsx', '**/__tests__/**'],
        rules: {
          'no-console': 'off',
        },
      },
    ],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
    },
  },
});
```

## `vpt stat-file .oxfmtrc.jsonc --assert-not file`

redundant standalone file removed

```
.oxfmtrc.jsonc: missing
```

## `vpt stat-file .oxlintrc.json --assert-not file`

redundant standalone file removed

```
.oxlintrc.json: missing
```

# migration_eslint_skipped_rule_warning

## `vp migrate --no-interactive`

Skipped ESLint rules should report warning

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
! Warnings:
  - Skipped 3 rules:
    - 3 Unsupported
      - camelcase: Superseded by `@typescript-eslint/naming-convention`, which accomplishes the same behavior with more flexibility.
      - array-bracket-newline: Deprecated stylistic rule, can be used via the stylistic eslint plugin as a JS Plugin if necessary.
      - no-invalid-this: Superseded by TypeScript's [`noImplicitThis`](https://www.typescriptlang.org/tsconfig/#noImplicitThis) compiler option (enabled by `strict` mode).
```

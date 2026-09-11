# migration_eslint_lint_staged_mjs

## `vp migrate --no-interactive`

migration should preserve non-JSON lint-staged config when hooks are not requested

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
• Skipped editor, hooks, and lint setup. Run `vp migrate --full` to apply them.
```

## `vpt print-file lint-staged.config.mjs`

verify non-JSON lint-staged config is preserved unchanged

```
export default {
  '*.ts': ['eslint --fix'],
};
```

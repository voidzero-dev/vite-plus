# migration_already_vite_plus_with_husky_lint_staged

## `git init`


## `vp migrate --no-interactive`

a version-update defers legacy husky/lint-staged setup to --full

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

## `vpt print-file package.json`

husky/lint-staged and prepare are left untouched

```
{
  "name": "migration-already-vite-plus-with-husky-lint-staged",
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "^9.1.7",
    "lint-staged": "^16.2.7",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "lint-staged": {
    "*": "vp check --fix"
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

## `vpt stat-file .husky --assert dir`

.husky is kept

```
.husky: dir
```

## `vp migrate --hooks --no-interactive`

--hooks opts into migrating legacy husky/lint-staged

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file package.json`

husky/lint-staged should be removed, prepare should be vp config

```
{
  "name": "migration-already-vite-plus-with-husky-lint-staged",
  "scripts": {
    "prepare": "vp config"
  },
  "devDependencies": {
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

## `vpt print-file .vite-hooks/pre-commit`

pre-commit hook should be rewritten

```
vp staged
```

## `vpt stat-file .husky --assert-not dir`

.husky should be removed

```
.husky: missing
```

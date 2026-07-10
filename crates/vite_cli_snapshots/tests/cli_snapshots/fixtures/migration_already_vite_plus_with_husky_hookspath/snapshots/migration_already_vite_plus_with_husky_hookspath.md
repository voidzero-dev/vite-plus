# migration_already_vite_plus_with_husky_hookspath

## `git init`


## `git config core.hooksPath .husky/_`


## `vp migrate --no-interactive`

a version-update defers legacy husky hooks to --full

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

## `git config --local core.hooksPath`

still husky's .husky/_ (not overridden)

```
.husky/_
```

## `vp migrate --hooks --no-interactive`

--hooks overrides husky's core.hooksPath and migrates hooks

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
  "name": "migration-already-vite-plus-with-husky-hookspath",
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

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```

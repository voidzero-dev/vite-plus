# migration_upgrade_hooks_standalone_staged_install_skip

## `git init`


## `git config core.hooksPath .custom-hooks`


## `vpt mkdir -p .custom-hooks`


## `vpt write-file .custom-hooks/pre-commit 'npx lint-staged
'`


## `vpt json-edit package.json devDependencies.lint-staged '"^16.2.7"'`


## `vpt write-file .lintstagedrc.json '{"*.ts":"eslint --fix"}
'`


## `vp migrate --hooks --no-interactive`

skip dispatcher installation

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
    vitest     0.1.21 → <version>
• Package manager settings configured
! Warnings:
  - Git hooks not configured — core.hooksPath is already set to ".custom-hooks", skipping
```

## `git config --local core.hooksPath`

keep the custom dispatcher

```
.custom-hooks
```

## `vpt print-file .custom-hooks/pre-commit`

keep the active hook

```
npx lint-staged
```

## `vpt print-file .lintstagedrc.json`

keep standalone config

```
{"*.ts":"eslint --fix"}
```

## `vpt print-file package.json`

keep lint-staged

```
{
  "devDependencies": {
    "lint-staged": "^16.2.7",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "onFail": "download",
      "version": "10.33.0"
    }
  },
  "name": "migration-upgrade-hooks-flag-pnpm",
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt stat-file vite.config.ts --assert missing`

roll back staged config

```
vite.config.ts: missing
```

## `git config --local --unset core.hooksPath`


## `vp migrate --hooks --no-interactive`

retry hook migration

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 3 config updates applied
• Git hooks configured
```

## `git config --local core.hooksPath`

install the dispatcher

```
.vite-hooks/_
```

## `vpt stat-file .lintstagedrc.json --assert missing`

remove standalone config

```
.lintstagedrc.json: missing
```

## `vpt print-file .vite-hooks/pre-commit`

keep the staged hook

```
vp staged
```

## `vpt print-file package.json`

remove lint-staged

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "onFail": "download",
      "version": "10.33.0"
    }
  },
  "name": "migration-upgrade-hooks-flag-pnpm",
  "scripts": {
    "prepare": "vp config"
  }
}
```

# migration_upgrade_hooks_disabled_preserves_policy

## `git init`


## `git config core.hooksPath .husky/_`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/pre-commit 'npx lint-staged
npm test
'`


## `vpt json-edit package.json scripts.prepare '"husky"'`


## `vpt json-edit package.json devDependencies.husky '"^9.1.7"'`


## `vpt json-edit package.json devDependencies.lint-staged '"^16.2.7"'`


## `vpt write-file .lintstagedrc.json '{"*.ts":"eslint --fix"}
'`


## `VITE_GIT_HOOKS=0 vp migrate --hooks --no-interactive`

skip disabled dispatcher installation

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
  - Git hooks not configured — skip install (git hooks disabled)
```

## `git config --local core.hooksPath`

restore the Husky dispatcher

```
.husky/_
```

## `vpt print-file .husky/pre-commit`

keep the project hook unchanged

```
npx lint-staged
npm test
```

## `vpt stat-file .vite-hooks/pre-commit --assert missing`

do not create a Vite+ hook

```
.vite-hooks/pre-commit: missing
```

## `vpt print-file .lintstagedrc.json`

keep standalone config

```
{"*.ts":"eslint --fix"}
```

## `vpt stat-file vite.config.ts --assert missing`

roll back staged config

```
vite.config.ts: missing
```

## `vpt print-file package.json`

keep hook dependencies and prepare script

```
{
  "devDependencies": {
    "husky": "^9.1.7",
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
    "prepare": "husky"
  }
}
```

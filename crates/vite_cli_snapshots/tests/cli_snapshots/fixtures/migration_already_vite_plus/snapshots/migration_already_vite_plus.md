# migration_already_vite_plus

## `vp migrate --no-interactive`

common existing project removes the stale wrapper override, no hooks/agent setup defaults

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vp migrate --no-interactive --hooks --agent agents`

explicit setup should still update existing vite-plus project

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite   → <version>
• 2 config updates applied
```

## `vpt print-file package.json`

prepare script should be configured for vp config

```
{
  "name": "migration-already-vite-plus",
  "devDependencies": {
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt stat-file AGENTS.md --assert file`

explicit agent instructions should be written

```
AGENTS.md: file
```

## `vpt stat-file .vite-hooks/pre-commit --assert file`

explicit pre-commit hook should be written

```
.vite-hooks/pre-commit: file
```

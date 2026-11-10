# migration_existing_vite_hooks

## `git init`


## `pnpm --version`


## `vp migrate --full --no-agent --no-editor`

Complete the existing hook setup without prompting, replacing project hooks, or changing echo arguments.

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Git hooks configured
• Package manager settings configured
```

## `vpt print-file package.json .vite-hooks/pre-commit .vite-hooks/commit-msg`

```
{
  "name": "migration-existing-vite-hooks",
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "prepare": "vp config && echo vp config --no-hooks"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
echo project-owned pre-commit
echo project-owned commit-msg
```

## `git config --local core.hooksPath`

```
.vite-hooks/_
```

## `vpt stat-file .vite-hooks/_/h --assert file`

```
.vite-hooks/_/h: file
```

## `vp migrate --full --no-agent --no-editor`

A complete hook setup is detected and skipped without prompting on the next migration.

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json .vite-hooks/pre-commit .vite-hooks/commit-msg`

```
{
  "name": "migration-existing-vite-hooks",
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "prepare": "vp config && echo vp config --no-hooks"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
echo project-owned pre-commit
echo project-owned commit-msg
```

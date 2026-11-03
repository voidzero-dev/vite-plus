# workspace_gitignore

Reuse the root .vitest/ rule without creating or changing workspace package ignore files.

## `vpt write-file pnpm-workspace.yaml 'packages:
  - packages/*
'`


## `vpt write-file .gitignore 'node_modules/
.vitest/
'`


## `vpt write-file packages/a/package.json '{"name":"a","private":true,"devDependencies":{"vitest":"4.1.11"}}
'`


## `vpt write-file packages/b/package.json '{"name":"b","private":true,"devDependencies":{"vitest":"4.1.11"}}
'`


## `vpt write-file packages/b/.gitignore 'dist/
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file .gitignore packages/b/.gitignore`

```
node_modules/
.vitest/
dist/
```

## `vpt stat-file packages/a/.gitignore --assert missing`

```
packages/a/.gitignore: missing
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file .gitignore packages/b/.gitignore`

```
node_modules/
.vitest/
dist/
```

## `vpt stat-file packages/a/.gitignore --assert missing`

```
packages/a/.gitignore: missing
```

# migration_ignore_patterns_baseurl_references

The external baseUrl fixer follows references and extends; skip it so excluded configs stay unchanged.

## `vpt write-file tsconfig.json '{ "compilerOptions": { "baseUrl": "." }, "references": [{ "path": "./templates/starter" }] }
'`

```
```

## `vpt write-file templates/starter/tsconfig.json '{ "extends": "../../tsconfig.json", "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }
'`

```
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 3 files had imports rewritten
• Package manager settings configured
! Warnings:
  - Skipped automatic tsconfig baseUrl removal because its external fixer cannot honor migrate.ignorePatterns. Remove baseUrl manually in migrated projects to enable type-aware linting.
```

## `vpt print-file templates/starter/tsconfig.json`

```
{ "extends": "../../tsconfig.json", "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }
```

## `vpt print-file tsconfig.json`

```
{ "compilerOptions": { "baseUrl": "." }, "references": [{ "path": "./templates/starter" }] }
```

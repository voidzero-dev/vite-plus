# migration_oxc_ecosystem_delta_comic

Reproduces the typed JSON imports in https://github.com/vite-plus-ecosystem-ci/delta-comic/pull/9.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file .oxlintrc.json`

```
{
  "rules": {
    "no-console": "error"
  },
  "options": {
    "typeAware": false,
    "typeCheck": false
  }
}
```

## `vpt print-file .oxfmtrc.json`

```
{
  "singleQuote": true,
  "semi": false
}
```

## `vp fmt packages/app/src`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file packages/app/src/index.ts`

```
export const message = 'preserved'
```

## `vp run check`

```
VITE+ - The Unified Toolchain for the Web

$ vp check packages/app/src ⊘ cache disabled
VITE+ - The Unified Toolchain for the Web

pass: All 1 file are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

repeat migration without invalidating either JSON import

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run check`

```
VITE+ - The Unified Toolchain for the Web

$ vp check packages/app/src ⊘ cache disabled
VITE+ - The Unified Toolchain for the Web

pass: All 1 file are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vpt write-file packages/app/src/index.ts 'console.log('\''hello'\'')
'`


## `vp run check`

vp check must still apply the imported lint rule

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ vp check packages/app/src ⊘ cache disabled
VITE+ - The Unified Toolchain for the Web

pass: All 1 file are correctly formatted (<duration>, <n> threads)
error: Lint issues found
× eslint(no-console): Unexpected console statement.
   ╭─[packages/app/src/index.ts:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 1 error and 0 warnings in 1 file (<duration>, <n> threads)
```

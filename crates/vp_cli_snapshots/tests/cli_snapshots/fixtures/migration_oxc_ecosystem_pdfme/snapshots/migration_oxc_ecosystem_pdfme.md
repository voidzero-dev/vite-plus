# migration_oxc_ecosystem_pdfme

Reproduces the root and workspace config flags and playground inheritance in https://github.com/vite-plus-ecosystem-ci/pdfme/pull/7.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
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

## `vpt print-file playground/.oxlintrc.json`

```
{
  "extends": ["../.oxlintrc.json"],
  "overrides": [
    {
      "files": ["e2e/**/*.ts"],
      "rules": {
        "no-console": "off"
      }
    }
  ]
}
```

## `vp run fmt`

```
VITE+ - The Unified Toolchain for the Web

~/packages/common$ vp fmt -c ../../.oxfmtrc.json src --write ⊘ cache disabled
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file packages/common/src/index.ts`

```
export const message = 'preserved'
```

## `vp run fmt:check`

```
VITE+ - The Unified Toolchain for the Web

$ vp fmt -c .oxfmtrc.json packages/common/src --check ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp run lint`

```
VITE+ - The Unified Toolchain for the Web

~/packages/common$ vp lint -c ../../.oxlintrc.json src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp run lint:root`

```
VITE+ - The Unified Toolchain for the Web

$ vp lint --config .oxlintrc.json packages/common/src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd playground && vp lint -c .oxlintrc.json src`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

repeat migration and rerun scripts from both the root and a workspace

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run fmt:check`

```
VITE+ - The Unified Toolchain for the Web

$ vp fmt -c .oxfmtrc.json packages/common/src --check ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `cd packages/common && vp run fmt`

```
VITE+ - The Unified Toolchain for the Web

~/packages/common$ vp fmt -c ../../.oxfmtrc.json src --write ⊘ cache disabled
Finished in <duration> on 1 files using <n> threads.
```

## `cd packages/common && vp run lint`

```
VITE+ - The Unified Toolchain for the Web

~/packages/common$ vp lint -c ../../.oxlintrc.json src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt write-file packages/common/src/index.ts 'console.log('\''hello'\'')
'`


## `vpt write-file playground/src/index.ts 'console.log('\''hello'\'')
'`


## `vp run lint`

the workspace script must still use the root JSON rules

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

~/packages/common$ vp lint -c ../../.oxlintrc.json src ⊘ cache disabled

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.ts:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd playground && vp lint -c .oxlintrc.json src`

the playground must still inherit the root JSON rules

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.ts:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

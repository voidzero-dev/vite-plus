# migration_oxc_ecosystem_pdfme_inline_config

Explicit config paths in pdfme-style scripts must also work when inline lint and fmt options exist.

## `vpt write-file vite.config.mts 'export default { lint: { rules: { '\''no-console'\'': '\''off'\'' } }, fmt: { singleQuote: false, semi: true } };
'`


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

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

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

## `vpt write-file packages/common/src/index.ts 'console.log('\''hello'\'')
'`


## `vp run lint`

the explicit JSON rule must override the inline no-console setting

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

## `vp run lint:root`

the root --config script must also use the explicit JSON rule

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ vp lint --config .oxlintrc.json packages/common/src ⊘ cache disabled

  × eslint(no-console): Unexpected console statement.
   ╭─[packages/common/src/index.ts:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

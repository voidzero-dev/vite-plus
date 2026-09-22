# migration_oxc_ecosystem_freecut

Reproduces the config file reads and script inputs in https://github.com/vite-plus-ecosystem-ci/freecut/pull/9.

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

## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ vp fmt src .oxlintrc.json .oxfmtrc.json ⊘ cache disabled
Finished in <duration> on 3 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = 'preserved'
```

## `vp run lint`

```
VITE+ - The Unified Toolchain for the Web

$ vp lint src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

repeat migration without deleting either file read by vite.config.ts

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run format:check`

```
VITE+ - The Unified Toolchain for the Web

$ vp fmt src .oxlintrc.json .oxfmtrc.json --check ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 3 files using <n> threads.
```

## `vp run lint`

```
VITE+ - The Unified Toolchain for the Web

$ vp lint src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt write-file src/index.ts 'console.log('\''hello'\'')
'`


## `vp run lint`

the rules read from the retained JSON must still apply

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ vp lint src ⊘ cache disabled

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.ts:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

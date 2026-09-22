# migration_existing_oxc_configs_unreferenced_lint

## `vpt write-file .oxlintrc.json '{"rules":{"no-console":"error"},"options":{"typeAware":false,"typeCheck":false}}
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

finish both unreferenced Oxc configs in a project that already uses Vite+

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 2 config updates applied
• Package manager settings configured
```

## `vpt stat-file .oxlintrc.json --assert-not file`

```
.oxlintrc.json: missing
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

```
.oxfmtrc.json: missing
```

## `vpt print-file vite.config.ts`

```
export default {
  fmt: {
    "singleQuote": true,
    "semi": false
  },
  lint: {"rules":{"no-console":"error","vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":false,"typeCheck":false},"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}]},
}
```

## `vp fmt src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = 'preserved'
```

## `vp lint src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

the restored finalization must remain a no-op on retry

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
export default {
  fmt: {
    "singleQuote": true,
    "semi": false
  },
  lint: {"rules":{"no-console":"error","vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":false,"typeCheck":false},"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}]},
}
```

## `vp fmt --check src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vpt write-file src/index.ts 'console.log('\''hello'\'')
'`


## `vp lint src/index.ts`

the merged lint rule must apply after the JSON config is removed

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

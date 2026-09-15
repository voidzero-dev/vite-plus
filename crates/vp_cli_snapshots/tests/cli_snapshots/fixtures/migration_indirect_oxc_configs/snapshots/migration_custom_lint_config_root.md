# migration_custom_lint_config_root

## `vp run lint:custom`

```
VITE+ - The Unified Toolchain for the Web

$ vp lint --config config/lint.json src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 config update applied
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

## `vpt print-file config/lint.json`

```
{
  "extends": ["../.oxlintrc.json"]
}
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

the unrelated formatter config still merges

```
.oxfmtrc.json: missing
```

## `vp run lint:custom`

the custom entrypoint must still load its inherited JSON config

```
VITE+ - The Unified Toolchain for the Web

$ vp lint --config config/lint.json src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt --check src/index.js`

```
VITE+ - The Unified Toolchain for the Web

Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run lint:custom`

```
VITE+ - The Unified Toolchain for the Web

$ vp lint --config config/lint.json src ⊘ cache disabled
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt write-file src/index.js 'console.log('\''hello'\'')
'`


## `vp run lint:custom`

the inherited no-console rule remains active after two migrations

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ vp lint --config config/lint.json src ⊘ cache disabled

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:1:1]
 1 │ console.log('hello')
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

# migration_existing_oxc_configs_extends

## `vpt write-file .oxlintrc.json '{"extends":["./lint-base.json"]}
'`


## `vpt write-file lint-base.json '{"rules":{"no-console":"error"}}
'`


## `vpt write-file src/index.ts 'console.log('\''hello'\'');
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

preserve JSON inheritance instead of copying file paths into inline lint.extends

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
{"extends":["./lint-base.json"]}
```

## `vp lint -c .oxlintrc.json src/index.ts`

the retained JSON config must still load the inherited no-console rule

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.ts:1:1]
 1 │ console.log('hello');
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

# migration_referenced_workspace_oxc_configs_extends

## `vpt write-file packages/app/.oxlintrc.json '{"extends":["./config/lint-base.jsonc"]}
'`


## `vpt write-file packages/app/config/lint-base.jsonc '// Shared lint rules
{"extends":["../../../.oxlintrc.json"]}
'`


## `vpt write-file packages/app/src/index.ts 'console.log('\''hello'\'');
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

preserve every config in a transitive JSON inheritance chain

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

## `vpt print-file .oxlintrc.json`

```
{
  "rules": {
    "no-console": "error"
  }
}
```

## `vpt print-file packages/app/.oxlintrc.json`

```
{"extends":["./config/lint-base.jsonc"]}
```

## `vpt print-file packages/app/config/lint-base.jsonc`

```
// Shared lint rules
{"extends":["../../../.oxlintrc.json"]}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

repeated migration keeps inherited rules usable

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `cd packages/app && vp lint -c .oxlintrc.json src/index.ts`

the inherited no-console rule must still report the console call

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

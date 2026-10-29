# migration_workspace_oxc_script_module

## `vpt json-edit package.json scripts.format 'node scripts/print-lint-rules.mjs'`


## `vpt write-file .oxlintrc.json '{"rules":{"no-console":"error"}}
'`


## `vpt write-file scripts/print-lint-rules.mjs 'import lint from '\''../.oxlintrc.json'\'' with { type: '\''json'\'' };
console.log(lint.rules['\''no-console'\'']);
'`


## `vp install --ignore-scripts`


## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ node scripts/print-lint-rules.mjs ⊘ cache disabled
error
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 4 config updates applied
• Package manager settings configured
```

## `vpt print-file .oxlintrc.json`

```
{"rules":{"no-console":"error"}}
```

## `vpt stat-file packages/other/.oxfmtrc.json --assert missing`

the unrelated workspace config still migrates

```
packages/other/.oxfmtrc.json: missing
```

## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ node scripts/print-lint-rules.mjs ⊘ cache disabled
error
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ node scripts/print-lint-rules.mjs ⊘ cache disabled
error
```

# migration_workspace_oxc_env_script_module

## `vpt json-edit package.json scripts.format 'env NODE_ENV=production node scripts/print-lint-rules.mjs'`


## `vpt write-file .oxlintrc.json '{"rules":{"no-console":"error"}}
'`


## `vpt write-file scripts/print-lint-rules.mjs 'import lint from '\''../.oxlintrc.json'\'' with { type: '\''json'\'' };
console.log(process.env.NODE_ENV, lint.rules['\''no-console'\'']);
'`


## `vp install --ignore-scripts`


## `vp run format`

```
$ env NODE_ENV=production node scripts/print-lint-rules.mjs ⊘ cache disabled
production error
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

## `vpt stat-file .oxlintrc.json --assert file`

```
.oxlintrc.json: file
```

## `vpt stat-file packages/other/.oxfmtrc.json --assert missing`

```
packages/other/.oxfmtrc.json: missing
```

## `vp run format`

```
$ env NODE_ENV=production node scripts/print-lint-rules.mjs ⊘ cache disabled
production error
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run format`

```
$ env NODE_ENV=production node scripts/print-lint-rules.mjs ⊘ cache disabled
production error
```

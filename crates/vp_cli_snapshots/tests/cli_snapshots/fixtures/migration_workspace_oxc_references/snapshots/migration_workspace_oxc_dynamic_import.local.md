# migration_workspace_oxc_dynamic_import

## `vpt write-file packages/app/vite.config.ts 'export default {};
'`


## `vpt write-file packages/other/vite.config.ts 'export default {};
'`


## `vpt write-file .oxfmtrc.json '{"singleQuote":true,"semi":false}
'`


## `vpt json-edit package.json scripts.format 'node inspect-config.mjs'`


## `vpt write-file helper.mjs 'import fmt from '\''./.oxfmtrc.json'\'' with { type: '\''json'\'' };
export default { define: { FORMAT_STYLE: fmt.singleQuote } };
'`


## `vpt write-file vite.config.ts 'const variant = '\''helper'\'';
const { default: config } = await import(`./${variant}.mjs`);
export default { ...config };
'`


## `vp install --ignore-scripts`


## `node inspect-config.mjs`

```
true
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

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

## `vpt print-file .oxfmtrc.json`

```
{"singleQuote":true,"semi":false}
```

## `vpt stat-file packages/other/.oxfmtrc.json --assert missing`

the unrelated workspace config still migrates

```
packages/other/.oxfmtrc.json: missing
```

## `node inspect-config.mjs`

```
true
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `node inspect-config.mjs`

```
true
```

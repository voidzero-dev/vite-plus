# migration_workspace_oxc_filesystem_cwd

## `vpt write-file packages/app/vite.config.ts 'export default {};
'`


## `vpt write-file packages/other/vite.config.ts 'export default {};
'`


## `vpt write-file .oxfmtrc.json '{"singleQuote":true,"semi":false}
'`


## `vpt json-edit package.json scripts.format 'node inspect-config.mjs'`


## `vpt write-file packages/app/helper.mjs 'import { readFileSync } from '\''node:fs'\'';
const fmt = JSON.parse(readFileSync('\''./.oxfmtrc.json'\'', '\''utf8'\''));
export default { define: { FORMAT_STYLE: fmt.singleQuote } };
'`


## `vpt write-file vite.config.ts 'import config from '\''./packages/app/helper.mjs'\'';
export default { ...config };
'`


## `vp install --ignore-scripts`


## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ node inspect-config.mjs ⊘ cache disabled
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

## `vpt stat-file packages/app/.oxfmtrc.json --assert missing`

the helper reads from its consumer cwd, not its own directory

```
packages/app/.oxfmtrc.json: missing
```

## `vpt stat-file packages/other/.oxfmtrc.json --assert missing`

the unrelated workspace config still migrates

```
packages/other/.oxfmtrc.json: missing
```

## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ node inspect-config.mjs ⊘ cache disabled
true
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

$ node inspect-config.mjs ⊘ cache disabled
true
```

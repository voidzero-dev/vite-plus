# migration_workspace_oxc_imported_chdir

## `vpt write-file packages/app/vite.config.ts 'export default {};
'`


## `vpt write-file packages/other/vite.config.ts 'export default {};
'`


## `vpt json-edit package.json scripts.format 'node inspect-config.mjs bundle'`


## `vpt write-file .oxfmtrc.json '{"singleQuote":false}
'`


## `vpt write-file change-cwd.mjs 'process.chdir('\''packages/app'\'');
'`


## `vpt write-file setup.mjs 'import '\''./change-cwd.mjs'\'';
'`


## `vpt write-file vite.config.ts 'import '\''./setup.mjs'\'';
import { readFileSync } from '\''node:fs'\'';
const fmt = JSON.parse(readFileSync('\''./.oxfmtrc.json'\'', '\''utf8'\''));
export default { define: { FORMAT_STYLE: fmt.singleQuote } };
'`


## `vp install --ignore-scripts`


## `node inspect-config.mjs bundle`

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
• Package manager settings configured
```

## `vpt stat-file packages/app/.oxfmtrc.json --assert file`

```
packages/app/.oxfmtrc.json: file
```

## `vpt stat-file .oxfmtrc.json --assert file`

```
.oxfmtrc.json: file
```

## `node inspect-config.mjs bundle`

```
true
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `node inspect-config.mjs bundle`

```
true
```

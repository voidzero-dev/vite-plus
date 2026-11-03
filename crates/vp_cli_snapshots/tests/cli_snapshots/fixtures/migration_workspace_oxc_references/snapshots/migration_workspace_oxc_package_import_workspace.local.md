# migration_workspace_oxc_package_import_workspace

## `vpt write-file packages/app/vite.config.ts 'export default {};
'`


## `vpt write-file packages/other/vite.config.ts 'export default {};
'`


## `vpt json-edit package.json scripts.format 'node inspect-config.mjs bundle'`


## `vpt write-file packages/app/helper.mjs 'import fmt from '\''./.oxfmtrc.json'\'' with { type: '\''json'\'' };
export default { FORMAT_STYLE: fmt.singleQuote };
'`


## `vpt json-edit package.json imports.#config ./packages/app/helper.mjs`


## `vpt write-file vite.config.ts 'import define from '\''#config'\'';
export default { define };
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

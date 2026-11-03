# migration_workspace_oxc_dynamic_workspace

## `vpt write-file packages/app/vite.config.ts 'export default {};
'`


## `vpt write-file packages/other/vite.config.ts 'export default {};
'`


## `vpt json-edit package.json scripts.format 'node inspect-config.mjs native vite.config.mjs'`


## `vpt write-file packages/app/helper.mjs 'import fmt from '\''./.oxfmtrc.json'\'' with { type: '\''json'\'' };
export default { FORMAT_STYLE: fmt.singleQuote };
'`


## `vpt rm vite.config.ts`


## `vpt write-file vite.config.mjs 'const app = '\''app'\'';
const { default: define } = await import(`./packages/${app}/helper.mjs`);
export default { define };
'`


## `vp install --ignore-scripts`


## `node inspect-config.mjs native vite.config.mjs`

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

## `node inspect-config.mjs native vite.config.mjs`

```
true
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `node inspect-config.mjs native vite.config.mjs`

```
true
```

# migration_workspace_oxc_package_manager_directory

## `vpt json-edit package.json scripts.format 'pnpm --dir packages/app exec vp fmt --check -c .oxfmtrc.json src/index.js'`


## `vp install --ignore-scripts`


## `vp run format`

```
$ pnpm --dir packages/app exec vp fmt --check -c .oxfmtrc.json src/index.js ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
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

## `vp install --ignore-scripts`


## `vpt print-file packages/app/.oxfmtrc.json`

```
{
  "singleQuote": true,
  "semi": false
}
```

## `vpt stat-file packages/other/.oxfmtrc.json --assert missing`

the unrelated workspace config still migrates

```
packages/other/.oxfmtrc.json: missing
```

## `vp run format`

```
$ pnpm --dir packages/app exec vp fmt --check -c .oxfmtrc.json src/index.js ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run format`

```
$ pnpm --dir packages/app exec vp fmt --check -c .oxfmtrc.json src/index.js ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

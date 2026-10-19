# migration_existing_oxc_configs_script

## `vpt json-edit package.json scripts.fmt 'vp fmt -c .oxfmtrc.json src/index.ts'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

preserve the config passed explicitly by a package script

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt stat-file .oxfmtrc.json --assert file`

```
.oxfmtrc.json: file
```

## `vpt print-file vite.config.ts`

```
export default {};
```

## `vp run fmt`

```
VITE+ - The Unified Toolchain for the Web

$ vp fmt -c .oxfmtrc.json src/index.ts ⊘ cache disabled
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = 'preserved'
```

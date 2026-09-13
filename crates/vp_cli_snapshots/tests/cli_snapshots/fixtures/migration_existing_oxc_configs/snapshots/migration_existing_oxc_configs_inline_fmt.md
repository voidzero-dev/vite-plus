# migration_existing_oxc_configs_inline_fmt

## `vpt write-file vite.config.ts 'export default { fmt: { singleQuote: false, semi: false } };
'`


## `vp fmt src/index.ts`

the existing inline fmt config takes precedence over the standalone config

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = "preserved"
```

## `vpt write-file src/index.ts 'export const message = "preserved";
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

remove the leftover standalone config without changing the existing fmt config

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file vite.config.ts`

```
export default { fmt: { singleQuote: false, semi: false } };
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

```
.oxfmtrc.json: missing
```

## `vp fmt src/index.ts`

formatting must still use the existing inline options after migration

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = "preserved"
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

retrying the completed migration should be a no-op

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
export default { fmt: { singleQuote: false, semi: false } };
```

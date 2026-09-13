# migration_existing_oxc_configs

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

finish a leftover Oxfmt config even when Vite+ is already installed

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 config update applied
• Package manager settings configured
```

## `vpt print-file vite.config.ts`

```
export default {
  fmt: {
    "singleQuote": true,
    "semi": false
  },

}
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

```
.oxfmtrc.json: missing
```

## `vp fmt src/index.ts`

the migrated options must affect formatting

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = 'preserved'
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a completed migration should be a no-op on retry

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
export default {
  fmt: {
    "singleQuote": true,
    "semi": false
  },

}
```

## `vpt stat-file AGENTS.md --assert-not file`

```
AGENTS.md: missing
```

## `vpt stat-file .vite-hooks --assert-not dir`

```
.vite-hooks: missing
```

## `vpt stat-file .vscode --assert-not dir`

```
.vscode: missing
```

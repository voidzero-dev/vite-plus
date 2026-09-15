# migration_existing_oxc_configs_imported

## `vpt write-file vite.config.ts 'import fmt from '\''./.oxfmtrc.json'\'' with { type: '\''json'\'' };
import { readFileSync } from '\''node:fs'\'';
const lint = JSON.parse(readFileSync(new URL('\''./.oxlintrc.json'\'', import.meta.url), '\''utf8'\''));
export default { fmt, lint };
'`


## `vpt write-file .oxlintrc.json '{"rules":{"no-debugger":"error"}}
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

preserve JSON configs loaded with imports and readFileSync

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file .oxfmtrc.json`

```
{
  "singleQuote": true,
  "semi": false
}
```

## `vpt print-file .oxlintrc.json`

```
{"rules":{"no-debugger":"error"}}
```

## `vp fmt src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.ts`

```
export const message = 'preserved'
```

## `vp lint src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

retry preserves both imported configs

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp fmt --check src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp lint src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

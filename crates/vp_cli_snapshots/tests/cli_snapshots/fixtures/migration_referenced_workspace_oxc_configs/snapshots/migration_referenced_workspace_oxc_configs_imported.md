# migration_referenced_workspace_oxc_configs_imported

## `vpt write-file packages/app/vite.config.ts 'import fmt from '\''../../.oxfmtrc.json'\'' with { type: '\''json'\'' };
import { readFileSync } from '\''node:fs'\'';
const lint = JSON.parse(readFileSync(new URL('\''../../.oxlintrc.json'\'', import.meta.url), '\''utf8'\''));
export default { fmt, lint };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

preserve root configs loaded by a workspace Vite config

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
  "singleQuote": true
}
```

## `vpt print-file .oxlintrc.json`

```
{
  "rules": {
    "no-console": "error"
  }
}
```

## `cd packages/app && vp fmt src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file packages/app/src/index.ts`

```
export const message = 'preserved';
```

## `cd packages/app && vp lint src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

repeated migration keeps the workspace imports usable

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `cd packages/app && vp fmt --check src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `cd packages/app && vp lint src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

# migration_workspace_oxc_bare_import

## `vpt json-edit package.json scripts.format 'vp fmt --check packages/app/src/index.js'`


## `vpt write-file vite.config.ts 'import fmt from '\''@repo/config'\'';
export default { fmt };
'`


## `vp install --ignore-scripts`


## `vp run format`

```
$ vp fmt --check packages/app/src/index.js ⊘ cache disabled
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

## `vpt print-file packages/app/.oxfmtrc.json`

```
{
  "singleQuote": true,
  "semi": false
}
```

## `vpt stat-file packages/other/.oxfmtrc.json --assert missing`

a bare workspace import does not preserve unrelated configs

```
packages/other/.oxfmtrc.json: missing
```

## `vpt print-file packages/other/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    "singleQuote": true,
    "semi": false
  },

});
```

## `vp run format`

the workspace package still imports its JSON config after migration

```
$ vp fmt --check packages/app/src/index.js ⊘ cache disabled
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
$ vp fmt --check packages/app/src/index.js ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

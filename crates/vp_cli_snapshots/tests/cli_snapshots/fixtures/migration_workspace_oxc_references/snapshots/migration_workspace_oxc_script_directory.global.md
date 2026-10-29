# migration_workspace_oxc_script_directory

## `vp install --ignore-scripts`


## `vp run format`

```
VITE+ - The Unified Toolchain for the Web

~/packages/app$ vp fmt --check -c .oxfmtrc.json src/index.js ⊘ cache disabled
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

the unrelated workspace config still migrates

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

the script still loads its explicit config after changing directories

```
VITE+ - The Unified Toolchain for the Web

~/packages/app$ vp fmt --check -c .oxfmtrc.json src/index.js ⊘ cache disabled
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
VITE+ - The Unified Toolchain for the Web

~/packages/app$ vp fmt --check -c .oxfmtrc.json src/index.js ⊘ cache disabled
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

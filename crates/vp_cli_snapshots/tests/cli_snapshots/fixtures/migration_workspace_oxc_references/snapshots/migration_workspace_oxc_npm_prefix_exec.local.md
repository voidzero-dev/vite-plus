# migration_workspace_oxc_npm_prefix_exec

## `vpt json-edit package.json scripts.format 'npm --offline --yes=false --prefix packages/app exec -- node scripts/read.mjs -c .oxfmtrc.json'`


## `vpt write-file .oxfmtrc.json '{"singleQuote":false}
'`


## `vpt write-file scripts/read.mjs 'import { readFileSync } from '\''node:fs'\'';
import path from '\''node:path'\'';
import { fileURLToPath } from '\''node:url'\'';
const root = fileURLToPath(new URL('\''../'\'', import.meta.url));
console.log(`cwd=${path.relative(root, process.cwd()) || '\''.'\''}`);
console.log(JSON.parse(readFileSync(process.argv.at(-1), '\''utf8'\'')).singleQuote);
'`


## `vp install --ignore-scripts`


## `node prepare-node-bin.mjs`


## `vp run format`

```
$ npm --offline --yes=false --prefix packages/app exec -- node scripts/read.mjs -c .oxfmtrc.json ⊘ cache disabled
cwd=.
false
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

## `vpt stat-file .oxfmtrc.json --assert file`

```
.oxfmtrc.json: file
```

## `vp run format`

```
$ npm --offline --yes=false --prefix packages/app exec -- node scripts/read.mjs -c .oxfmtrc.json ⊘ cache disabled
cwd=.
false
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp run format`

```
$ npm --offline --yes=false --prefix packages/app exec -- node scripts/read.mjs -c .oxfmtrc.json ⊘ cache disabled
cwd=.
false
```

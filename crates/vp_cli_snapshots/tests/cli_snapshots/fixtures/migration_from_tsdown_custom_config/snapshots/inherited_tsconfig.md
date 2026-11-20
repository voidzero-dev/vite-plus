# inherited_tsconfig

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.typescript 7.0.2`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\''; import pack from '\''./tsdown.config.ts'\''; export default defineConfig({ pack });'`


## `vpt mkdir -p configs`


## `vpt write-file configs/node-base.json '{"include":["../vite.config.ts","../tsdown.config.ts"]}'`


## `vpt write-file tsconfig.node.json '{
  "extends": "./configs/node-base.json",
  "compilerOptions": {
    "composite": true,
    "noEmit": true,
    "module": "NodeNext",
    "allowImportingTsExtensions": true,
    "skipLibCheck": true,
    "target": "ES2022"
  }
}
'`


## `vp install --ignore-scripts`


## `vp exec tsc -b tsconfig.node.json --force`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vp exec tsc -b tsconfig.node.json --force`


## `vpt print-file configs/node-base.json`

```
{ "include": ["../vite.config.ts", "../tsdown.config.ts"] }
```

## `vpt print-file tsconfig.node.json`

```
{
  "extends": "./configs/node-base.json",
  "compilerOptions": {
    "composite": true,
    "noEmit": true,
    "module": "NodeNext",
    "allowImportingTsExtensions": true,
    "skipLibCheck": true,
    "target": "ES2022"
  },
  "include": ["vite.config.ts", "tsdown.config.ts", "vite-pack.config.ts"]
}
```

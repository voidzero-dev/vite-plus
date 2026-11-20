# composite_tsconfig

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.typescript 7.0.2`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\''; import pack from '\''./tsdown.config.ts'\''; export default defineConfig({ pack });'`


## `vpt write-file tsconfig.node.json '{
  "compilerOptions": {
    "composite": true,
    "noEmit": true,
    "module": "NodeNext",
    "allowImportingTsExtensions": true,
    "skipLibCheck": true,
    "target": "ES2022"
  },
  "include": ["vite.config.ts", "tsdown.config.ts"]
}
'`


## `vp install --ignore-scripts`


## `vp exec tsc -b tsconfig.node.json --force`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file vite-pack.config.ts --assert file`

```
vite-pack.config.ts: file
```

## `vp exec tsc -b tsconfig.node.json --force`


## `vpt print-file tsconfig.node.json`

```
{
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

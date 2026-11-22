# composite_package_preset

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.typescript 7.0.2`


## `vpt write-file tsdown.config.mjs 'export default { entry: ['\''src/library.ts'\''], publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\''; import pack from '\''./tsdown.config.mjs'\''; export default defineConfig({ pack });'`


## `vpt mkdir -p configs`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''.'\'', configs]
catalogs:
  build:
    tsdown: 0.23.4
'`


## `vpt write-file configs/package.json '{"name":"@repo/tsconfig","version":"1.0.0","exports":{"./node.json":"./node-base.json"}}'`


## `vpt json-edit package.json devDependencies.@repo/tsconfig workspace:*`


## `vpt write-file configs/node-base.json '{"compilerOptions":{"allowJs":true},"files":["${configDir}/vite.config.ts","${configDir}/tsdown.config.mjs"]}'`


## `vpt write-file tsconfig.node.json '{
  "extends": "@repo/tsconfig/node.json",
  "compilerOptions": {
    "composite": true,
    "noEmit": true,
    "module": "NodeNext",
    "skipLibCheck": true,
    "target": "ES2022"
  }
}
'`


## `vp install --ignore-scripts`


## `vp exec tsc -b tsconfig.node.json --force`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vp exec tsc -b tsconfig.node.json --force`


## `vpt stat-file vite-pack.config.mjs --assert file`

```
vite-pack.config.mjs: file
```

## `vpt print-file configs/node-base.json`

```
{
  "compilerOptions": { "allowJs": true },
  "files": ["${configDir}/vite.config.ts", "${configDir}/tsdown.config.mjs"]
}
```

## `vpt print-file tsconfig.node.json`

```
{
  "extends": "@repo/tsconfig/node.json",
  "compilerOptions": {
    "composite": true,
    "noEmit": true,
    "module": "NodeNext",
    "skipLibCheck": true,
    "target": "ES2022"
  },
  "include": ["${configDir}/vite-pack.config.mjs"]
}
```

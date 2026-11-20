# configdir_descendant

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.typescript 7.0.2`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''.'\'', '\''packages/*'\'']
catalogs:
  build:
    tsdown: 0.23.4
'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\''; import pack from '\''./tsdown.config.ts'\''; export default defineConfig({ pack });'`


## `vpt mkdir -p configs packages/member/src`


## `vpt write-file configs/base.json '{
  "compilerOptions": {
    "composite": true,
    "noEmit": true,
    "target": "ES2022",
    "module": "NodeNext",
    "skipLibCheck": true,
    "allowImportingTsExtensions": true
  },
  "include": ["${configDir}/src", "${configDir}/vite.config.ts", "${configDir}/tsdown.config.ts"]
}
'`


## `vpt write-file tsconfig.json '{"extends":"./configs/base.json"}'`


## `vpt write-file packages/member/package.json '{"name":"member","private":true,"type":"module"}'`


## `vpt write-file packages/member/tsconfig.json '{"extends":"../../tsconfig.json"}'`


## `vpt write-file packages/member/src/member.ts 'export const member = true;'`


## `vp install --ignore-scripts`


## `vp exec tsc --showConfig -p packages/member/tsconfig.json`

The inherited source paths are relative to the member.

```
VITE+ - The Unified Toolchain for the Web

{
    "compilerOptions": {
        "allowImportingTsExtensions": true,
        "composite": true,
        "module": "nodenext",
        "noEmit": true,
        "skipLibCheck": true,
        "target": "es2022",
        "moduleResolution": "nodenext",
        "moduleDetection": "force",
        "declaration": true,
        "incremental": true
    },
    "files": [
        "./src/member.ts"
    ],
    "include": [
        "<workspace>/packages/member/src",
        "<workspace>/packages/member/vite.config.ts",
        "<workspace>/packages/member/tsdown.config.ts"
    ]
}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vp exec tsc -b --force tsconfig.json packages/member/tsconfig.json`


## `vpt stat-file vite-pack.config.ts --assert file`

```
vite-pack.config.ts: file
```

## `vpt print-file tsconfig.json`

```
{
  "extends": "./configs/base.json",
  "include": [
    "${configDir}/src",
    "${configDir}/vite.config.ts",
    "${configDir}/tsdown.config.ts",
    "${configDir}/vite-pack.config.ts"
  ]
}
```

## `vp exec tsc --showConfig -p packages/member/tsconfig.json`

The member still lists its own source after migration.

```
VITE+ - The Unified Toolchain for the Web

{
    "compilerOptions": {
        "allowImportingTsExtensions": true,
        "composite": true,
        "module": "nodenext",
        "noEmit": true,
        "skipLibCheck": true,
        "target": "es2022",
        "moduleResolution": "nodenext",
        "moduleDetection": "force",
        "declaration": true,
        "incremental": true
    },
    "files": [
        "./src/member.ts"
    ],
    "include": [
        "<workspace>/packages/member/src",
        "<workspace>/packages/member/vite.config.ts",
        "<workspace>/packages/member/tsdown.config.ts",
        "<workspace>/packages/member/vite-pack.config.ts"
    ]
}
```

# focused_npx_no_install_vpx

npx --no-install finds vpx after installing only the workspace member.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - '\''packages/*'\''
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","generate":"npx --no-install vpx ./generate.ts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;
'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };
'`


## `vpt write-file packages/generator/generate.ts 'const value: string = '\''LOCAL_VPX_RAN'\'';
console.log(value);
'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `pnpm --filter generator install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `vpt stat-file packages/generator/node_modules/vite-plus/package.json --assert file`

```
packages/generator/node_modules/vite-plus/package.json: file
```

## `cd packages/generator && npm_config_offline=true pnpm run generate`

```
$ npx --no-install vpx ./generate.ts
LOCAL_VPX_RAN
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```

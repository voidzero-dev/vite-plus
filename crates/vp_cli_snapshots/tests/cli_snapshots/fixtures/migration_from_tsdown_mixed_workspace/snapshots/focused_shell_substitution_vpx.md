# focused_shell_substitution_vpx

A command substitution uses the vpx binary from the member install.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - '\''packages/*'\''
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src packages/generator/scripts`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","build":"BUILD_VERSION=$(vpx ./scripts/version.ts) node ./build.mjs"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;
'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };
'`


## `vpt write-file packages/generator/scripts/version.ts 'const version: string = '\''1.2.3'\'';
console.log(version);
'`


## `vpt write-file packages/generator/build.mjs 'if (process.env.BUILD_VERSION '\!'== '\''1.2.3'\'') throw new Error('\''Cannot stamp the build version'\'');
console.log('\''VERSION_STAMP='\'' + process.env.BUILD_VERSION);
'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `pnpm --filter generator install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `vpt stat-file packages/generator/node_modules/.bin/vpx --assert file`

```
packages/generator/node_modules/.bin/vpx: file
```

## `cd packages/generator && pnpm run build`

```
$ BUILD_VERSION=$(vpx ./scripts/version.ts) node ./build.mjs
VERSION_STAMP=1.2.3
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```

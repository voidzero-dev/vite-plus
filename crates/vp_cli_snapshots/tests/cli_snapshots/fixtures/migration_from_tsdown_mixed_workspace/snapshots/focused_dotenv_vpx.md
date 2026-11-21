# focused_dotenv_vpx

A dotenv-wrapped script finds the local vpx after an install of only its workspace member.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - '\''packages/*'\''
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","generate":"dotenv -e .env -- vpx ./generate.ts"},"devDependencies":{"tsdown":"0.22.14","dotenv-cli":"8.0.0"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;
'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };
'`


## `vpt write-file packages/generator/.env 'FIXTURE_OUTPUT=WORKSPACE_LOCAL
'`


## `vpt write-file packages/generator/generate.ts 'const value: string = process.env.FIXTURE_OUTPUT ?? '\'''\'';
if (value '\!'== '\''WORKSPACE_LOCAL'\'') throw new Error('\''dotenv did not load the member environment'\'');
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

## `cd packages/generator && pnpm run generate`

```
$ dotenv -e .env -- vpx ./generate.ts
WORKSPACE_LOCAL
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```

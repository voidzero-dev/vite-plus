# focused_retained_lint_staged

A member install supplies vpx to its retained lint-staged config.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","tsx":"4.20.3","lint-staged":"16.3.1"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt mkdir -p packages/generator/scripts`


## `vpt write-file packages/generator/.lintstagedrc.json '{"src/*.ts":"tsx ./scripts/generate.ts"}'`


## `vpt write-file packages/generator/scripts/generate.ts 'import { writeFileSync } from '\''node:fs'\'';
writeFileSync('\''hook-ran'\'', '\''run'\'');
'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","lint-staged":"16.3.1","vite-plus":"catalog:"}}
```

## `pnpm --filter generator install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `vpt stat-file packages/generator/node_modules/vite-plus/package.json --assert file`

```
packages/generator/node_modules/vite-plus/package.json: file
```

## `vpt print-file packages/generator/.lintstagedrc.json`

```
{"src/*.ts":"vpx ./scripts/generate.ts"}
```

## `git init -q`


## `git -c user.name=Fixture -c user.email=fixture@example.invalid -c core.hooksPath=.git/no-hooks -c commit.gpgsign=false commit --allow-empty -m 'Fixture baseline'`


## `git add packages/generator/src/index.ts`


## `cd packages/generator && pnpm exec lint-staged --no-stash --concurrent=false`


## `vpt print-file packages/generator/hook-ran`

```
run
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```

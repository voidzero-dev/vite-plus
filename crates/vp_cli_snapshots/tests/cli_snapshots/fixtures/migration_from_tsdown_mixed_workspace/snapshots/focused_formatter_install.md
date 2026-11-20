# focused_formatter_install

## `vpt json-edit package.json devDependencies '{"tsdown":"catalog:build"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
catalogs:
  build:
    tsdown: 0.23.4
'`


## `vpt mkdir -p packages/format-member/src`


## `vpt write-file packages/format-member/package.json '{"name":"format-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.ts"},"devDependencies":{"tsdown":"0.23.4","typescript":"7.0.2"}}'`


## `vpt write-file packages/format-member/release.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''format-out'\'', dts: false };'`


## `vpt write-file packages/format-member/src/library.ts 'export const value = 42;'`


## `vpt write-file packages/format-member/.oxfmtrc.json '{"singleQuote":true}'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/format-member/package.json packages/format-member/vite.config.ts`

```
{"name":"format-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.ts"},"devDependencies":{"tsdown":"0.23.4","typescript":"7.0.2","vite-plus":"catalog:"}}import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {"singleQuote":true},

});
```

## `pnpm --filter format-member install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/format-member && vp exec tsc --ignoreConfig --noEmit --skipLibCheck --module NodeNext vite.config.ts`


## `cd packages/format-member && vp run release`


## `vpt stat-file packages/format-member/format-out/library.mjs --assert file`

```
packages/format-member/format-out/library.mjs: file
```

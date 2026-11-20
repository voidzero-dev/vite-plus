# focused_vitest_config_install

A filtered install provides Vite+ for a member that previously used vitest/config.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8","vitest":"5.1.1"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/vitest-member/src`


## `vpt write-file packages/vitest-member/package.json '{"name":"vitest-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/vitest-member/vite.config.ts 'import { defineConfig } from '\''vitest/config'\''; export default defineConfig({});'`


## `vpt write-file packages/vitest-member/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file packages/vitest-member/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/vitest-member/index.html '<main>Member test config</main>'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/vitest-member/package.json packages/vitest-member/vite.config.ts`

```
{"name":"vitest-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}import { defineConfig } from 'vite-plus'; export default defineConfig({});
```

## `pnpm --filter vitest-member install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/vitest-member && node --input-type=module -e 'console.log(typeof (await import('\''./vite.config.ts'\'')).default)'`

```
object
```

## `cd packages/vitest-member && vp build --configLoader native`


## `vpt print-file packages/vitest-member/dist/index.html`

```
<main>Member test config</main>
```

## `cd packages/vitest-member && vp run release`


## `vpt stat-file packages/vitest-member/release-out/index.mjs --assert file`

```
packages/vitest-member/release-out/index.mjs: file
```

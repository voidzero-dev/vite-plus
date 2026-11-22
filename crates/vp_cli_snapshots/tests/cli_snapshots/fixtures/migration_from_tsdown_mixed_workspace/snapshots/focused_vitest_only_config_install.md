# focused_vitest_only_config_install

A member with its own Vitest config can load it after a filtered install and still publish with tsdown.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8","vitest":"5.1.1"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/test-member/src`


## `vpt write-file packages/test-member/package.json '{"name":"test-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/test-member/vitest.config.ts 'import { defineConfig } from '\''vitest/config'\''; export default defineConfig({ test: { globals: true } });'`


## `vpt write-file packages/test-member/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file packages/test-member/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/test-member/src/example.spec.ts 'test('\''loads the member config'\'', () => expect(1 + 1).toBe(2));'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/test-member/package.json packages/test-member/vitest.config.ts`

```
{"name":"test-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}import { defineConfig } from 'vite-plus'; export default defineConfig({ test: { globals: true } });
```

## `pnpm --filter test-member install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/test-member && node --input-type=module -e 'console.log((await import('\''./vitest.config.ts'\'')).default.test.globals)'`

```
true
```

## `cd packages/test-member && vp test run --config vitest.config.ts`


## `cd packages/test-member && vp run release`


## `vpt stat-file packages/test-member/release-out/index.mjs --assert file`

```
packages/test-member/release-out/index.mjs: file
```

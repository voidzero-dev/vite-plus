# yarn_without_project_vite_or_vitest

Install only vite-plus: its dependency graph must provide Vitest's Vite peer without a project Vite or Vitest dependency.

## `vpt write-file package.json '{"name":"test-v5-yarn-peer","private":true,"type":"module","packageManager":"yarn@4.12.0","devDependencies":{"vite-plus":"latest"}}
'`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
'`


## `vp install`


## `node --input-type=module -e 'import assert from '\''node:assert/strict'\''; import { createRequire } from '\''node:module'\''; import { defineConfig } from '\''vite-plus/test/config'\''; const require = createRequire(import.meta.resolve('\''vite-plus/package.json'\'')); assert.equal(require('\''vite/package.json'\'').name, '\''@voidzero-dev/vite-plus-core'\''); assert.equal(require('\''vite/package.json'\'').version, require('\''@voidzero-dev/vite-plus-core/package.json'\'').version); assert.equal(typeof defineConfig, '\''function'\''); console.log('\''Vitest Vite peer uses the bundled core'\'');'`

```
Vitest Vite peer uses the bundled core
```

## `vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓ identity.test.js (1 test) <duration>
   ✓ installed runner shares custom matchers and mock state <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

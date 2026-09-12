import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

function typecheck(source) {
  writeFileSync('fixture.ts', source);
  const result = spawnSync(
    process.execPath,
    [
      'node_modules/typescript/bin/tsc',
      '--noEmit',
      '--strict',
      '--pretty',
      'false',
      '--skipLibCheck',
      'false',
      '--moduleResolution',
      'bundler',
      '--module',
      'esnext',
      '--target',
      'es2022',
      '--types',
      'node',
      'fixture.ts',
    ],
    { encoding: 'utf8' },
  );
  assert.equal(result.error, undefined);
  assert.equal(result.stderr, '');
  return result;
}

if (process.argv[2] === 'pack') {
  const integration = `import * as attw from '@arethetypeswrong/core';
import * as volarTypeScript from '@volar/typescript';
const pack = {
  entry: 'src/index.ts',
  attw: { module: attw },
  dts: { customLanguages: [{
    extensionPatterns: [/\\.vue$/], volarTypeScript, createVolarPlugins: () => [],
  }] },
};
export default defineConfig({ pack });`;
  for (const imports of [
    "import { defineConfig } from 'vite-plus';",
    "import { defineConfig } from 'vite-plus/config'; import type {} from 'vite-plus/pack';",
  ]) {
    const result = typecheck(`${imports}\n${integration}`);
    assert.equal(result.status, 0, result.stdout);
  }
  const invalid = typecheck(`import { defineConfig } from 'vite-plus/config';
import type { PackUserConfig } from 'vite-plus/pack';
const pack: PackUserConfig = { entry: 42 };
export default defineConfig({ pack });`);
  assert.notEqual(invalid.status, 0);
  assert.match(invalid.stdout, /Type 'number' is not assignable/);
  assert.doesNotMatch(invalid.stdout, /node_modules|Cannot find module/);

  const { defineConfig } = await import('vite-plus');
  const attw = await import('@arethetypeswrong/core');
  const volarTypeScript = await import('@volar/typescript');
  const pack = { attw: { module: attw }, dts: { customLanguages: [{ volarTypeScript }] } };
  const result = defineConfig({ pack });
  assert.equal(result.pack, pack);
  assert.equal(result.pack.attw.module, attw);
  assert.equal(result.pack.dts.customLanguages[0].volarTypeScript, volarTypeScript);
  console.log(
    'Original and explicit pack configuration preserve injected module types and identity',
  );
} else {
  for (const name of [
    '@arethetypeswrong/core',
    '@vitejs/devtools/cli-commands',
    'publint',
    'unplugin-unused',
    '@volar/typescript',
  ]) {
    assert.throws(() => require.resolve(name), { code: 'MODULE_NOT_FOUND' });
  }
  const result = typecheck(readFileSync('vite.config.ts', 'utf8'));
  assert.equal(result.status, 0, result.stdout);

  for (const [config, message] of [
    ["pack: { entry: 'src/index.ts' }", /'pack' does not exist/],
    ["fmt: { semi: 'yes' }", /not assignable to type 'boolean/],
  ]) {
    const invalid = typecheck(`import { defineConfig } from 'vite-plus/config';
export default defineConfig({ ${config} });`);
    assert.notEqual(invalid.status, 0);
    assert.match(invalid.stdout, message);
    assert.doesNotMatch(invalid.stdout, /node_modules|Cannot find module/);
  }

  const root = await import('vite-plus');
  const config = await import('vite-plus/config');
  assert.deepEqual(Object.keys(config), Object.keys(root));
  for (const name of Object.keys(root)) {
    assert.equal(config[name], root[name], name);
  }
  assert.equal(require('vite-plus/config'), require('vite-plus'));
  const plugin = { name: 'consumer' };
  const configured = config.defineConfig({ plugins: [plugin] });
  assert.deepEqual(
    configured.plugins.map(({ name }) => name),
    [
      'vite-plus:vitest-resolver',
      'vite-plus:auto-inline-matcher-deps',
      'vite-plus:coverage-version-guard',
      'consumer',
    ],
  );
  assert.equal(configured.plugins.at(-1), plugin);
  console.log(
    'App configuration is strict without pack; ESM, CommonJS, and native plugins retain identity',
  );
}

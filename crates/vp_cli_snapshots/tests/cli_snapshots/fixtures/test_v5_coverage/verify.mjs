import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { relative, resolve } from 'node:path';
import { defineConfig } from 'vite-plus';
import { createVitest } from 'vite-plus/test/node';

const readJson = (file) => JSON.parse(readFileSync(file, 'utf8'));
const expectedFiles = ['src/covered.js', 'src/untested.js'];
const files = (map, root = process.cwd()) => Object.keys(map).map((file) => relative(root, file).replaceAll('\\', '/')).sort();
const config = (provider) => defineConfig({ test: {
  include: ['coverage.test.js'],
  coverage: {
    enabled: true, provider, reporter: ['json'],
    include: ['src/**'], exclude: ['src/ignored.js'],
    reportsDirectory: `output/${provider}`,
    thresholds: { perFile: true, 'src/**': { perFile: true, lines: 0 } },
  },
} });
for (const provider of ['v8', 'istanbul']) {
  const runner = await createVitest({ config: false, watch: false, reporters: [] }, config(provider));
  try {
    assert.equal(runner.config.coverage.thresholds['src/**'].perFile, true);
    const result = await runner.start();
    assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
    assert.equal(runner.state.getCountOfFailedTests(), 0);
    const map = readJson(`output/${provider}/coverage-final.json`);
    assert.deepEqual(files(map), expectedFiles);
    assert.ok(Object.values(map[resolve('src/covered.js')].s).some((hits) => hits > 0));
    assert.ok(Object.values(map[resolve('src/untested.js')].s).every((hits) => hits === 0));
    console.log(`${provider}: matching provider, shared runner, covered/untested files, and exclusion passed`);
  } finally { await runner.close(); }

  for (const perFile of [false, true]) {
    const threshold = spawnSync(process.execPath, ['node_modules/vite-plus/dist/bin.js', 'test', 'run', '--config', 'threshold.config.js'], {
      encoding: 'utf8', env: { ...process.env, COVERAGE_PROVIDER: provider, PER_FILE: String(perFile) },
    });
    assert.equal(threshold.status, perFile ? 1 : 0, `${threshold.stdout}\n${threshold.stderr}`);
    if (perFile) {
      assert.match(`${threshold.stdout}\n${threshold.stderr}`, /untested\.js/);
      assert.match(`${threshold.stdout}\n${threshold.stderr}`, /threshold/);
    }
  }
  console.log(`${provider}: aggregate glob threshold passes; perFile rejects the untested file`);
}

const legacy = spawnSync(process.execPath, ['node_modules/vitest/vitest.mjs', 'run'], {
  cwd: 'legacy', encoding: 'utf8', env: process.env,
});
assert.equal(legacy.status, 0, `${legacy.stdout}\n${legacy.stderr}`);
const legacyFiles = files(readJson('legacy/coverage/coverage-final.json'), resolve('legacy'));
assert.deepEqual(legacyFiles, expectedFiles);
console.log('Vitest v4/v5 coverage file sets match: src/covered.js, src/untested.js');

const require = createRequire(import.meta.url);
const istanbul = readJson(require.resolve('@vitest/coverage-istanbul/package.json'));
for (const name of ['coverage', 'instrument', 'report', 'source-maps']) {
  assert.ok(istanbul.dependencies[`@vitest/istanbul-lib-${name}`]);
}
console.log('Istanbul provider uses the final @vitest/istanbul-lib-* graph');

// Corrupt only this disposable install's provider metadata. The runner must
// reject the mismatch before loading or executing the provider.
const manifest = require.resolve('@vitest/coverage-v8/package.json');
const before = readFileSync(manifest, 'utf8');
try {
  writeFileSync(manifest, JSON.stringify({ ...JSON.parse(before), version: '4.1.11' }));
  await assert.rejects(
    () => createVitest({ config: false, watch: false, reporters: [] }, config('v8')),
    /coverage provider must match the test runner version/,
  );
  console.log('Mismatched provider rejected before tests run');
} finally { writeFileSync(manifest, before); }

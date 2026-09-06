import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { defineConfig } from 'vite-plus';
import { createVitest, resolveConfig } from 'vite-plus/test/node';

const base = { config: false, watch: false, reporters: [] };
const resolved = await resolveConfig(base, defineConfig({ test: { clearMocks: false } }));
assert.equal(resolved.test.clearMocks, false);
assert.ok(resolved.plugins.some(({ name }) => name === 'vite-plus:vitest-resolver'));
console.log('resolveConfig returns Vite config with .test and integration plugins');

for (const staticParse of [true, false]) {
  const runner = await createVitest({ ...base, include: ['dynamic.test.js'] }, defineConfig({}));
  try {
    const result = await runner.collect([], { staticParse });
    assert.equal(result.unhandledErrors.length, 0);
    assert.equal(existsSync('collection-ran.txt'), !staticParse);
    const tasks = runner.state.getFiles().flatMap((file) => file.tasks);
    if (!staticParse) assert.deepEqual(tasks.map(({ name }) => name), ['generated at collection time']);
    console.log(`collect staticParse=${staticParse}: collection side effects ${staticParse ? 'skipped' : 'executed'}`);
  } finally {
    await runner.close();
  }
}

const runner = await createVitest({
  ...base,
  include: ['runtime.test.js'],
  benchmark: { enabled: true, include: ['benchmark.test.js'] },
  reporters: ['json', 'junit', 'blob'],
}, defineConfig({}));
try {
  const result = await runner.start();
  assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
  assert.equal(runner.state.getCountOfFailedTests(), 0, JSON.stringify(runner.state.getFiles().flatMap((file) => [file.result, ...file.tasks.map((task) => ({ name: task.name, result: task.result }))])));
  const tasks = runner.state.getFiles().flatMap((file) => file.tasks);
  assert.equal(tasks.length, 4);
  assert.equal(runner.mode, 'test');
  const artifacts = tasks.flatMap((task) => task.artifacts ?? []);
  assert.ok(artifacts.some(({ type }) => type === 'fixture:identity'));
  const attachment = tasks.flatMap((task) => task.annotations).find(({ attachment }) => attachment)?.attachment;
  assert.ok(attachment?.path.includes('.vitest/attachments/'));
  assert.equal(readFileSync(attachment.path, 'utf8'), 'v5 attachment content');
} finally {
  await runner.close();
}

const reports = '.vitest';
const json = JSON.parse(readFileSync(join(reports, 'json/output.json'), 'utf8'));
assert.equal(json.numPassedTests, 4);
assert.equal(json.numFailedTests, 0);
assert.match(readFileSync(join(reports, 'junit/output.xml'), 'utf8'), /tests="4"/);
assert.equal(readdirSync(join(reports, 'blob')).filter((file) => file.endsWith('.json')).length, 1);
console.log('JSON, JUnit, blob, attachments, artifacts, worker IDs, class mock prototypes, and the bench fixture passed');

const merged = await createVitest({ ...base, reporters: [['json', { outputFile: 'merged.json' }]] }, defineConfig({}));
try {
  const result = await merged.mergeReports(join(reports, 'blob'));
  assert.equal(result.unhandledErrors.length, 0);
  assert.equal(JSON.parse(readFileSync('merged.json', 'utf8')).numPassedTests, 4);
  console.log('blob merge preserved all four test results');
} finally {
  await merged.close();
}

import assert from 'node:assert/strict';
import { createVitest } from 'vite-plus/test/node';

const runner = await createVitest({ watch: false, reporters: [], benchmark: { enabled: true } });
try {
  const result = await runner.start();
  assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
  const files = runner.state.getFiles();
  assert.equal(files.length, 1);
  assert.equal(files[0].result.state, 'pass', JSON.stringify(files[0].result));
  const tests = files[0].tasks[0].tasks;
  assert.deepEqual(tests.map(({ name, mode, result }) => [name, result?.state ?? mode]), [
    ['parse', 'pass'],
    ['async', 'pass'],
    ['skipped', 'skip'],
    ['later', 'todo'],
  ]);
  assert.equal(tests[0].benchmarks.length, 1);
  assert.equal(tests[1].benchmarks.length, 1);
  console.log('Vitest 5 executed both migrated benchmarks and preserved skip/todo modifiers');
} finally {
  await runner.close();
}

import assert from 'node:assert/strict';
import path from 'node:path';
import { createVitest } from 'vite-plus/test/node';

// The production import must still work without Vitest globals.
const { add } = await import('./src/add.js');
assert.equal(add(2, 3), 5);

const runner = await createVitest({ watch: false, reporters: [] });
try {
  const result = await runner.start();
  assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
  const files = runner.state.getFiles();
  assert.deepEqual(
    files.map(file => path.relative(process.cwd(), file.filepath).replaceAll('\\', '/')).sort(),
    ['control.test.js', 'src/add.js'],
  );
  for (const file of files) {
    assert.equal(file.result?.state, 'pass', JSON.stringify(file.result));
    assert.equal(file.tasks.length, 1);
    assert.equal(file.tasks[0].result?.state, 'pass');
    assert.equal(!!file.tasks[0].concurrent, false);
  }
  console.log('Vitest 5 passed the in-source test and normal control; production import preserved');
} finally {
  await runner.close();
}

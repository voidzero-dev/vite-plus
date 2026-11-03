import assert from 'node:assert/strict';
import { realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { createVitest } from 'vite-plus/test/node';

const root = createRequire(import.meta.url);
const child = createRequire(new URL('./packages/child/package.json', import.meta.url));
const vitePlus = (require) => realpathSync(require.resolve('vite-plus/package.json'));
const vitest = (require) => realpathSync(createRequire(vitePlus(require)).resolve('vitest/package.json'));
assert.notEqual(vitePlus(root), vitePlus(child));
assert.notEqual(vitest(root), vitest(child));
console.log('Root and child have distinct Vite+ and Vitest peer instances');

// Check programmatic runners too: selecting an instance must not depend on
// environment variables set only by the vp CLI.
for (const mode of ['single', 'shared', 'separate']) {
  process.env.IDENTITY_MODE = mode;
  const runner = await createVitest({ watch: false, reporters: [] });
  try {
    const project = runner.projects[0];
    assert.equal(project.sharedViteServer, mode === 'shared');
    const result = await runner.start();
    assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
    assert.equal(runner.state.getCountOfFailedTests(), 0, JSON.stringify(runner.state.getFiles().map(file => file.result?.errors)));
    assert.equal(runner.state.getFiles().length, 1);
    console.log(`${mode}: collection, hooks, assertions, and mocks passed`);
  } finally {
    await runner.close();
  }
}

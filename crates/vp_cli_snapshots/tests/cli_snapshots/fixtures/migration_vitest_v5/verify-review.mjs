import assert from 'node:assert/strict';
import path from 'node:path';
import { createVitest } from 'vite-plus/test/node';

const scopeCheck = process.argv.includes('--scopes');
const dirIndex = process.argv.indexOf('--dir');
const runner = await createVitest({ config: scopeCheck ? undefined : './vite.config.ts', ...(dirIndex >= 0 ? { dir: process.argv[dirIndex + 1] } : {}), watch: false, reporters: [] });
try {
  if (scopeCheck) {
    assert.equal(runner.projects.length, 1);
    assert.equal(runner.projects[0].config.globals, true);
    if (process.argv.includes('--root')) {
      // Vitest uses forward slashes even on Windows; compare native paths.
      assert.equal(path.normalize(runner.projects[0].config.root), path.join(process.cwd(), 'unit'));
    }
    const result = await runner.start();
    assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
    assert.equal(runner.state.getCountOfFailedTests(), 0, JSON.stringify(runner.state.getFiles().map(file => file.result)));
    assert.equal(runner.state.getFiles().length, 1);
    assert.equal(runner.state.getFiles()[0].tasks.length, 1);
    console.log('Vitest 5 passed the migrated globals-only test and setup hook in the selected scope');
  } else if (process.argv.includes('--inheritance')) {
    const project = runner.getProjectByName('unit');
    assert.equal(project.config.clearMocks, true);
    assert.equal(project.config.browser.locators.exact, true);
    console.log('External base settings preserved: clearMocks=true, browser.locators.exact=true');
  } else {
    const result = await runner.collect([], { staticParse: false });
    assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
    const tasks = runner.state.getFiles().flatMap((file) => file.tasks);
    assert.equal(tasks.length, 2);
    // The collector leaves concurrent undefined for sequential tasks.
    assert.deepEqual(tasks.map(({ name, timeout, concurrent }) => ({ name, timeout, concurrent: !!concurrent })), [
      { name: 'slow test', timeout: 15000, concurrent: false },
      { name: 'slow it', timeout: 12000, concurrent: false },
    ]);
    console.log('Runtime collection preserved sequential timeouts: test=15000, it=12000');
  }
} finally {
  await runner.close();
}

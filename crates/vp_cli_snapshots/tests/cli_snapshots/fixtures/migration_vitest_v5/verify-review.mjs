import assert from 'node:assert/strict';
import { createVitest } from 'vite-plus/test/node';

const runner = await createVitest({ config: './vite.config.ts', watch: false, reporters: [] });
try {
  if (process.argv.includes('--inheritance')) {
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

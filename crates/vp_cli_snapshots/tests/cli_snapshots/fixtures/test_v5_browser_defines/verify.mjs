import assert from 'node:assert/strict';
import { realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { isAbsolute, relative } from 'node:path';
import { inspect } from 'node:util';
import { createVitest } from 'vite-plus/test/node';
import { configFor, project } from './config.mjs';

if (process.argv.includes('--packed')) {
  const require = createRequire(import.meta.resolve('vite-plus/package.json'));
  for (const name of ['vite-plus', 'vite', 'vitest', '@vitest/browser-playwright']) {
    const installed = relative(realpathSync(process.cwd()), realpathSync(require.resolve(`${name}/package.json`)));
    assert.ok(!isAbsolute(installed) && !installed.startsWith('..'), `${name} must come from the packed installation: ${installed}`);
  }
}

for (const [name, count] of [['raw', 1], ['helper', 1], ['shared', 2], ['shared-node', 2], ['inherited', 2], ['separate', 2], ['independent', 3], ['referenced', 2], ['nested', 2], ['injected', 3], ['injected-hook', 2]]) {
  const runner = await createVitest({ config: false, watch: false, reporters: [] }, configFor(name));
  try {
    if (name === 'injected') {
      await runner.injectTestProject([
        { extends: false, ...project('injected-browser') },
        { extends: false, ...project('injected-node', false) },
      ]);
    }
    assert.equal(runner.projects.length, count, name);
    const servers = new Set(runner.projects.map((project) => project.vite));
    if (name === 'shared' || name === 'shared-node') assert.equal(servers.size, 1);
    if (name === 'separate') assert.equal(servers.size, 2);
    const result = await runner.start();
    assert.equal(result.unhandledErrors.length, 0, inspect(result.unhandledErrors));
    const files = runner.state.getFiles();
    assert.equal(files.length, count, name);
    for (const file of files) {
      assert.equal(file.tasks.length, 3);
      assert.equal(file.result.state, 'pass', inspect(file.tasks.map((task) => task.result), { depth: 5 }));
      for (const task of file.tasks) assert.equal(task.result.state, 'pass', inspect(task.result));
    }
    console.log(`${name}: ${count} projects; string, boolean, number, object, expression, and dotted defines passed`);
  } finally {
    await runner.close();
  }
}

import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { defineConfig } from 'vite-plus';
import { createVitest } from 'vite-plus/test/node';
import { preview } from 'vite-plus/test/browser-preview';

// Automate Preview's normal manually opened page, without replacing its
// locator implementation or opening the developer's default browser.
const require = createRequire(import.meta.resolve('vite-plus/package.json'));
const { chromium } = require('playwright');
const browser = await chromium.launch({ headless: true });
const tab = await browser.newPage();
const provider = preview();
const realTimers = process.argv.includes('--real-timers');
const browserDefines = process.argv.includes('--browser-defines');
const createProvider = provider.providerFactory;
provider.providerFactory = (project) => {
  const instance = createProvider(project);
  const openPage = instance.openPage.bind(instance);
  instance.openPage = async (session, url) => {
    project.browser.vite.openBrowser = () => {};
    await openPage(session, url);
    await tab.goto(url);
  };
  return instance;
};
let runner;
try {
  runner = await createVitest({ config: false, watch: false, reporters: [] }, defineConfig({
    ...(browserDefines ? { define: { __VP_STRING_DEFINE__: JSON.stringify('/messages'), __VP_BOOL_DEFINE__: 'false' } } : {}),
    test: {
      include: [browserDefines ? 'defines.test.js' : realTimers ? 'upstream.test.js' : 'browser.test.js'],
      browser: { enabled: true, provider, headless: false, instances: [{ browser: 'chromium' }] },
    },
  }));
  const result = await runner.start();
  assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
  assert.equal(runner.state.getFiles().length, 1);
  if (browserDefines) {
    const tasks = runner.state.getFiles().flatMap((file) => file.tasks);
    assert.equal(tasks.length, 2);
    for (const task of tasks) {
      assert.equal(task.result.state, 'pass', JSON.stringify(task.result));
    }
    console.log('Preview: browser string and boolean define values passed');
  } else if (realTimers) {
    const tasks = runner.state.getFiles().flatMap((file) => file.tasks);
    assert.equal(tasks.length, 1);
    assert.equal(tasks[0].result.state, 'fail');
    assert.match(tasks[0].result.errors[0].message, /timers APIs are not mocked/);
    console.log('Release blocker reproduced: upstream Vitest 5.0.0 Preview locator clicks fail with real timers');
    // The assertion above deliberately verifies an upstream failure. Do not
    // treat this case as evidence that the Preview release gate passed.
    process.exitCode = 0;
  } else {
    assert.equal(runner.state.getCountOfFailedTests(), 0, JSON.stringify(runner.state.getFiles().flatMap((file) => [file.result, ...file.tasks.map((task) => task.result)])));
    console.log('Preview: runtime identity, browser export aliases, fake-timer locator clicks, and async assertions passed');
  }
} finally {
  await runner?.close();
  await browser.close();
}

import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { defineConfig } from 'vite-plus';
import { createVitest } from 'vite-plus/test/node';
import { webdriverio } from 'vite-plus/test/browser-webdriverio';

const require = createRequire(import.meta.resolve('vite-plus/package.json'));
const { chromium } = require('playwright');
assert.equal(require('@vitest/browser-webdriverio/package.json').version, '5.0.0-rc.1');
const provider = webdriverio({ capabilities: { 'goog:chromeOptions': {
  binary: chromium.executablePath(), args: ['--no-sandbox'],
} } });
let sessionOpened = false;
const createProvider = provider.providerFactory;
provider.providerFactory = (project) => {
  const instance = createProvider(project);
  const openPage = instance.openPage.bind(instance);
  instance.openPage = async (session, url) => {
    assert.equal(new URL(url).searchParams.get('sessionId'), session);
    sessionOpened = true;
    return openPage(session, url);
  };
  return instance;
};
const runner = await createVitest({ config: false, watch: false, reporters: [] }, defineConfig({ test: {
  include: ['browser.test.js'],
  browser: {
    enabled: true, headless: true, provider, instances: [{ browser: 'chrome' }],
    viewport: { width: 800, height: 600 },
    commands: { inspectLocator(context, locator) {
      assert.equal(typeof locator, 'object');
      return { ...locator, session: typeof context.sessionId === 'string' && context.sessionId.length > 0 };
    } },
  },
} }));
try {
  const result = await runner.start();
  assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
  assert.equal(runner.state.getCountOfFailedTests(), 0, JSON.stringify(runner.state.getFiles().flatMap((file) => [file.result, ...file.tasks.map((task) => task.result)])));
  assert.equal(runner.state.getFiles().length, 1);
  assert.ok(sessionOpened);
  console.log('WebDriverIO 5.0.0-rc.1 with Vitest 5.0.0: runner/context identity, real-timer clicks, session URLs, and serialized custom-command locators passed');
} finally { await runner.close(); }

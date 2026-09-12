import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { defineConfig } from 'vite-plus';
import { createVitest } from 'vite-plus/test/node';
import { playwright } from 'vite-plus/test/browser-playwright';

const headed = process.argv.includes('--headed');
const pngs = (directory) => readdirSync(directory, { recursive: true }).filter((name) => name.endsWith('.png')).map((name) => join(directory, name));
const dimensions = (file) => {
  const buffer = readFileSync(file);
  assert.equal(buffer.subarray(1, 4).toString(), 'PNG');
  return [buffer.readUInt32BE(16), buffer.readUInt32BE(20)];
};
for (const phase of ['record', 'compare', 'failure']) {
  const runner = await createVitest({
    config: false, watch: false, reporters: [], update: phase === 'record',
  }, defineConfig({ test: {
    include: [phase === 'failure' ? 'failure.test.js' : 'browser.test.js'],
    browser: {
      enabled: true, provider: playwright(), headless: !headed, ui: false,
      instances: [{ browser: 'chromium' }], viewport: { width: 800, height: 600 },
      screenshotFailures: true, screenshotDirectory: 'manual-images',
      expect: { toMatchScreenshot: { screenshotDirectory: 'reference-images' } },
    },
  } }));
  try {
    const result = await runner.start();
    assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
    assert.equal(runner.state.getFiles().length, 1);
    if (phase === 'failure') {
      assert.equal(runner.state.getFiles()[0].tasks.length, 1);
      const task = runner.state.getFiles()[0].tasks[0];
      assert.equal(task.result.state, 'fail');
      assert.equal(task.result.errors.length, 1);
      assert.match(task.result.errors[0].message, /fixture deliberately fails/);
      const images = pngs('.vitest/attachments/failure-screenshots');
      assert.equal(images.length, 1);
      assert.deepEqual(dimensions(images[0]), [800, 600]);
      console.log('Failure screenshot: .vitest/attachments/failure-screenshots, 800 x 600');
    } else {
      assert.equal(runner.state.getCountOfFailedTests(), 0,
        JSON.stringify(runner.state.getFiles().flatMap((file) => file.tasks.map((task) => task.result))));
      const references = pngs('reference-images');
      assert.equal(references.length, 1, `reference images: ${JSON.stringify(references)}`);
      assert.deepEqual(dimensions(references[0]), [100, 40]);
      const manual = pngs('manual-images').filter((file) => file.includes('viewport'));
      assert.equal(manual.length, 1, `manual images: ${JSON.stringify(pngs('manual-images'))}`);
      assert.deepEqual(dimensions(manual[0]), [800, 600]);
      assert.equal(existsSync('__screenshots__'), false);
      console.log(`${headed ? 'Headed' : 'Headless'} ${phase}: custom reference path, 100 x 40 element, 800 x 600 viewport`);
    }
  } finally { await runner.close(); }
}
// All assertions above must succeed, including the deliberate failure's error
// and attachment. No unexpected browser failure can turn this case green.
process.exitCode = 0;

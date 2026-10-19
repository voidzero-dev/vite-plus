import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

// Use the ecosystem project's installed packages, not the repository's links.
const require = createRequire(`${process.cwd()}/`);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const { createServer }: typeof import('vite') = await import(
  pathToFileURL(cliRequire.resolve('vite')).href
);
const { chromium }: typeof import('playwright') = require('playwright');

const server = await createServer({
  root: resolve('apps/dashboard'),
  server: { host: '127.0.0.1', port: 0 },
});

try {
  assert.equal(server.config.experimental.bundledDev, true);
  await server.listen();
  const url = server.resolvedUrls?.local[0];
  assert.ok(url, 'The bundled dev server must have a local URL');

  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const errors: string[] = [];
    page.on('pageerror', (error) => {
      errors.push(error.message);
      console.error(error);
    });
    // Font downloads are unrelated to bundled dev and need no external service.
    await page.route('https://fonts.googleapis.com/**', (route) =>
      route.fulfill({ contentType: 'text/css', body: '' }),
    );

    const [, runtime] = await Promise.all([
      page.goto(url),
      page.waitForResponse(
        (response) => new URL(response.url()).pathname === '/@rolldown/experimental-runtime.mjs',
      ),
    ]);
    assert.equal(runtime.status(), 200);
    await page.getByRole('heading', { name: 'VoidZero Dashboard', exact: true }).waitFor();
    assert.deepEqual(errors, [], 'The dashboard must render without browser errors');
    console.log('ok vibe-dashboard renders in Chromium with bundled dev enabled');
  } finally {
    await browser.close();
  }
} finally {
  await server.close();
}

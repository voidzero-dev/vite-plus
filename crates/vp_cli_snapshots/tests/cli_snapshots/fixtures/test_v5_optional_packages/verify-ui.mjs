import assert from 'node:assert/strict';
import { defineConfig } from 'vite-plus';
import { createVitest } from 'vite-plus/test/node';

const runner = await createVitest({
  config: false, watch: false, reporters: [], ui: true, open: false,
  api: { host: '127.0.0.1' },
}, defineConfig({
  plugins: [{
    name: 'ui-server-port',
    // Vitest replaces a plain api.port: 0 with its default. Override it last.
    enforce: 'post',
    config: {
      order: 'post',
      handler(config) {
        config.test.api.port = 0;
      },
    },
  }],
}));
try {
  assert.equal(runner.config.api.port, 0);
  if (!runner.vite.httpServer.listening) await runner.vite.listen(0);
  const base = `http://127.0.0.1:${runner.vite.httpServer.address().port}`;
  const ui = new URL(runner.config.uiBase, base);
  assert.equal((await fetch(ui)).status, 403);
  ui.searchParams.set('token', runner.config.api.token);
  const authenticated = await fetch(ui, { redirect: 'manual' });
  assert.equal(authenticated.status, 302);
  const cookie = authenticated.headers.get('set-cookie');
  assert.match(cookie, /HttpOnly/);
  assert.match(cookie, /SameSite=Strict/i);
  const clean = new URL(authenticated.headers.get('location'), base);
  assert.equal(clean.search, '');
  const page = await fetch(clean, { headers: { cookie } });
  assert.equal(page.status, 200);
  assert.ok((await page.text()).includes(JSON.stringify(runner.config.api.token)));
  console.log('UI rejects a bare URL, accepts its token, and serves the clean URL with an authenticated cookie');
} finally { await runner.close(); }

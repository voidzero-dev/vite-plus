import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, it } from 'vite-plus/test';

const coreDir = fileURLToPath(new URL('..', import.meta.url));

it.each(['missing', 'unrelated'])(
  'serves the bundled dev runtime when the rolldown package is %s',
  (rolldownPackage) => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-bundled-dev-runtime-'));
    try {
      // Copy the package outside the checkout so its devDependency on rolldown
      // cannot hide an unresolved runtime import. Install it under the `vite`
      // alias used by vite-plus to also exercise package self-references.
      const installedCore = path.join(root, 'node_modules/vite');
      fs.mkdirSync(installedCore, { recursive: true });
      fs.copyFileSync(path.join(coreDir, 'package.json'), path.join(installedCore, 'package.json'));
      fs.cpSync(path.join(coreDir, 'dist'), path.join(installedCore, 'dist'), { recursive: true });
      const pkg = JSON.parse(fs.readFileSync(path.join(coreDir, 'package.json'), 'utf8'));
      for (const name of Object.keys({ ...pkg.dependencies, ...pkg.optionalDependencies })) {
        const source = path.join(coreDir, 'node_modules', name);
        if (fs.existsSync(source)) {
          const dest = path.join(root, 'node_modules', name);
          fs.mkdirSync(path.dirname(dest), { recursive: true });
          fs.symlinkSync(fs.realpathSync(source), dest, 'junction');
        }
      }

      if (rolldownPackage === 'unrelated') {
        const unrelatedDir = path.join(root, 'node_modules/rolldown');
        fs.mkdirSync(unrelatedDir, { recursive: true });
        fs.writeFileSync(
          path.join(unrelatedDir, 'package.json'),
          JSON.stringify({
            name: 'rolldown',
            type: 'module',
            exports: { './experimental/runtime': './experimental-runtime.mjs' },
          }),
        );
        fs.writeFileSync(
          path.join(unrelatedDir, 'experimental-runtime.mjs'),
          'throw new Error("Unrelated rolldown runtime");',
        );
      }

      fs.writeFileSync(
        path.join(root, 'index.html'),
        '<script type="module" src="/main.js"></script>',
      );
      fs.writeFileSync(path.join(root, 'main.js'), 'document.body.textContent = "bundled dev";');
      const script = path.join(installedCore, 'verify.mjs');
      fs.writeFileSync(
        script,
        `
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createServer } from '@voidzero-dev/vite-plus-core';

const server = await createServer({
  configFile: false,
  root: process.cwd(),
  base: '/nested/',
  logLevel: 'silent',
  experimental: { bundledDev: true },
  server: { host: '127.0.0.1', port: 0 },
});
try {
  await server.listen();
  const base = 'http://127.0.0.1:' + server.httpServer.address().port + '/nested/';
  const client = await fetch(base + 'bundledDevClient.mjs');
  assert.equal(client.status, 200);
  assert.match(await client.text(), /from\\s*['"]\\/nested\\/@rolldown\\/experimental-runtime\\.mjs['"]/);
  const runtime = await fetch(base + '@rolldown/experimental-runtime.mjs');
  assert.equal(runtime.status, 200);
  const expectedRuntime = fs.readFileSync(
    new URL(import.meta.resolve('@voidzero-dev/vite-plus-core/rolldown/experimental/runtime')), 'utf8',
  );
  assert.ok(await runtime.text() === expectedRuntime, 'must serve the bundled rolldown runtime');
  console.log('served bundled runtime and rewrote client import');
} finally {
  await server.close();
}
`,
      );
      const env = { ...process.env };
      delete env.NODE_PATH;
      expect(
        execFileSync(process.execPath, [script], {
          cwd: root,
          env,
          encoding: 'utf8',
          timeout: 30_000,
        }),
      ).toBe('served bundled runtime and rewrote client import\n');
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  },
  30_000,
);

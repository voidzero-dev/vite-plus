import assert from 'node:assert/strict';
import { readFileSync, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, sep } from 'node:path';

export default {
  base: '/nested/',
  clearScreen: false,
  experimental: {
    bundledDev: true,
  },
  plugins: [
    {
      name: 'verify-bundled-dev-runtime',
      configureServer(server) {
        const projectRequire = createRequire(join(server.config.root, 'package.json'));
        const cliRequire = createRequire(projectRequire.resolve('vite-plus/package.json'));
        const corePackage = cliRequire.resolve('vite/package.json');
        // Reject a linked checkout: its development dependencies can mask #3110.
        assert.ok(realpathSync(corePackage).startsWith(realpathSync('node_modules') + sep));
        const coreRequire = createRequire(corePackage);
        assert.equal(coreRequire('./package.json').name, '@voidzero-dev/vite-plus-core');
        if (process.env.ROLLDOWN_PACKAGE === 'unrelated') {
          assert.equal(
            coreRequire.resolve('rolldown/experimental/runtime'),
            realpathSync('node_modules/rolldown/experimental-runtime.mjs'),
          );
        } else {
          assert.throws(() => coreRequire.resolve('rolldown/experimental/runtime'), {
            code: 'MODULE_NOT_FOUND',
          });
        }
        const expectedRuntime = readFileSync(
          coreRequire.resolve('@voidzero-dev/vite-plus-core/rolldown/experimental/runtime'),
          'utf8',
        );
        server.httpServer?.once('listening', () => {
          // Let Vite print its startup banner after server.listen() resolves.
          setImmediate(async () => {
            try {
              const base = `http://127.0.0.1:${server.httpServer.address().port}/nested/`;
              const client = await fetch(base + 'bundledDevClient.mjs');
              assert.equal(client.status, 200);
              assert.match(
                await client.text(),
                /from\s*['"]\/nested\/@rolldown\/experimental-runtime\.mjs['"]/,
              );
              const runtime = await fetch(base + '@rolldown/experimental-runtime.mjs');
              assert.equal(runtime.status, 200);
              assert.ok((await runtime.text()) === expectedRuntime, 'must serve core\'s runtime');
              console.log(`Rolldown package: ${process.env.ROLLDOWN_PACKAGE}`);
              console.log('Client imports /nested/@rolldown/experimental-runtime.mjs');
              console.log('Served runtime matches core');
              const name = Buffer.from('dev-server:runtime-verified').toString('base64url');
              process.stdout.write(`\x1b]2;vt-pty-terminal-test:${'0'.repeat(32)}:${name}\x1b\\`);
            } catch (error) {
              console.error(error);
              process.exit(1);
            }
          });
        });
      },
    },
  ],
};

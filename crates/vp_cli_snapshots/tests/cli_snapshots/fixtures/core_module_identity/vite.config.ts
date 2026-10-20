import assert from 'node:assert/strict';

import { isRunnableDevEnvironment } from 'vite';
import { defineConfig, isRunnableDevEnvironment as fromVitePlus } from 'vite-plus';

export default defineConfig({
  server: { host: '127.0.0.1', port: 0 },
  plugins: [
    {
      name: 'check-ssr-environment',
      configureServer(server) {
        // Vitest also loads this config, with an in-memory server.
        const httpServer = server.httpServer;
        if (!httpServer) {
          return;
        }
        assert.equal(isRunnableDevEnvironment, fromVitePlus);
        assert.ok(isRunnableDevEnvironment(server.environments.ssr));
        httpServer.once('listening', async () => {
          try {
            const address = httpServer.address();
            assert.ok(address && typeof address === 'object');
            const response = await fetch(`http://127.0.0.1:${address.port}/`, {
              signal: AbortSignal.timeout(10_000),
            });
            assert.equal(response.status, 200);
            assert.equal(await response.text(), 'SSR middleware installed');
            console.log('SSR environment identity and HTTP response passed');
          } catch (error) {
            console.error(error);
            process.exitCode = 1;
          } finally {
            await server.close();
          }
        });
        // TanStack installs its middleware after this guard. A duplicate core
        // makes the guard fail, leaving the root route unhandled.
        return () => {
          if (isRunnableDevEnvironment(server.environments.ssr)) {
            server.middlewares.use((_req, res) => res.end('SSR middleware installed'));
          }
        };
      },
    },
  ],
});

import assert from 'node:assert/strict';

const build = {
  outDir: 'output',
};

export default {
  clearScreen: false,
  build,
  preview: {
    host: '127.0.0.1',
    port: 0,
  },
  plugins: [
    {
      name: 'check-preview-output',
      configurePreviewServer(server) {
        server.httpServer.once('listening', async () => {
          try {
            const address = server.httpServer.address();
            assert.ok(address && typeof address === 'object');
            const response = await fetch(`http://127.0.0.1:${address.port}/`, {
              signal: AbortSignal.timeout(10_000),
            });
            assert.equal(response.status, 200);
            assert.match(await response.text(), /<h1>Preview output<\/h1>/);
            console.log('Configured build output served HTTP 200');
          } catch (error) {
            console.error(error);
            process.exitCode = 1;
          } finally {
            await server.close();
          }
        });
      },
    },
  ],
};

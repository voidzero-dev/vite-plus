import assert from 'node:assert/strict';

export default {
  server: { host: '127.0.0.1', port: 0 },
  preview: { host: '127.0.0.1', port: 0 },
  plugins: [
    {
      name: 'check-target-server',
      configureServer: checkServer,
      configurePreviewServer: checkServer,
    },
  ],
};

function checkServer(server) {
  server.httpServer.once('listening', async () => {
    try {
      const address = server.httpServer.address();
      assert.ok(address && typeof address === 'object');
      const response = await fetch(`http://127.0.0.1:${address.port}/`, {
        signal: AbortSignal.timeout(10_000),
      });
      assert.equal(response.status, 200);
      assert.match(await response.text(), /Correct project root/);
      console.log('The target app served HTTP 200');
    } catch (error) {
      console.error(error);
      process.exitCode = 1;
    } finally {
      // PreviewServer has no close() method.
      if (server.close) {
        await server.close();
      } else {
        server.httpServer.close();
      }
    }
  });
}

import { expect, test } from 'vite-plus/test';
import { createServer, isRunnableDevEnvironment } from 'vite-plus';

test('the bundled server shares the public Vite API', async () => {
  const server = await createServer({
    configFile: false,
    // This API identity check does not need a shared WebSocket listening port.
    server: { middlewareMode: true, ws: false },
  });
  try {
    expect(isRunnableDevEnvironment(server.environments.ssr)).toBe(true);
  } finally {
    await server.close();
  }
});

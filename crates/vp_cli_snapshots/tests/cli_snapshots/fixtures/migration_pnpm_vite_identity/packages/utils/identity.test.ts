import { expect, test } from 'vite-plus/test';
import { createServer, isRunnableDevEnvironment } from 'vite-plus';

test('the bundled server shares the public Vite API', async () => {
  const server = await createServer({ configFile: false, server: { middlewareMode: true } });
  try {
    expect(isRunnableDevEnvironment(server.environments.ssr)).toBe(true);
  } finally {
    await server.close();
  }
});

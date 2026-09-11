import { expect, test } from 'vite-plus/test';
import { createServer, isRunnableDevEnvironment } from 'vite-plus';

test('the public guard accepts an environment created by the bundled server', async () => {
  const server = await createServer({ configFile: false, server: { middlewareMode: true } });
  try {
    expect(isRunnableDevEnvironment(server.environments.ssr)).toBe(true);
  } finally {
    await server.close();
  }
});

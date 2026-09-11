import { createServer } from 'vite';

// A Vite-core programmatic API. `vite-plus` does not re-expose it on its public
// surface, so this import (and the type position below) must stay on `vite`
// rather than be rewritten to `vite-plus` (issue #2004).
export type ViteApi = Pick<typeof import('vite'), 'createBuilder' | 'loadConfigFromFile'>;

export async function start() {
  const server = await createServer();
  await server.listen();
}

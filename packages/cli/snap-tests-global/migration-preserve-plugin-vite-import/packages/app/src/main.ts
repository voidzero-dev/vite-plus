import { createServer } from 'vite';

export async function start() {
  const server = await createServer();
  await server.listen();
}

import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';

if (process.platform !== 'win32') {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(path.join(tmpdir(), 'supervisor.sock'), resolve);
  });
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

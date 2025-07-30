import { existsSync } from 'node:fs';
import { join } from 'node:path';

try {
  const vpPath = join(process.cwd(), 'node_modules/vite-plus/binding/index.js');
  if (!existsSync(vpPath)) {
    throw new Error('vite-plus is not installed in the current project');
  }
  const { run } = await import(vpPath);
  run();
} catch (e) {
  const error = new Error('Failed to run vite-plus');
  error.cause = e;
  throw error;
}

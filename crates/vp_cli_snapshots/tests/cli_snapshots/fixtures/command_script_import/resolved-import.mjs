import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const register = createRequire(import.meta.url).resolve('vite-plus/register');
const preload = process.argv[2] === 'path' ? register : pathToFileURL(register).href;
console.log('preload:', preload.replaceAll('\\', '/'));
const child = spawnSync(process.execPath, ['--import', preload, './src/name #%.ts'], {
  stdio: 'inherit',
  timeout: 30000,
});
if (child.error) throw child.error;
process.exitCode = child.status ?? 1;

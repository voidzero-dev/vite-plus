import { spawnSync } from 'node:child_process';
import path from 'node:path';

const env = { ...process.env };
const pathKey = Object.keys(env).find((key) => key.toLowerCase() === 'path') ?? 'PATH';
env[pathKey] = [path.resolve('fake-bin'), env[pathKey]].filter(Boolean).join(path.delimiter);

const result = spawnSync('vp', ['env', 'clean'], { env, stdio: 'inherit' });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);

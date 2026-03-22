import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';

const [command, ...args] = process.argv.slice(2);

if (!command) {
  console.error('Usage: node packages/tools/src/with-curl-http1.ts <command> [args...]');
  process.exit(1);
}

let tempDir: string | undefined;
const env = { ...process.env };

if (process.platform === 'darwin') {
  tempDir = mkdtempSync(join(tmpdir(), 'vite-plus-curl-'));
  const curlPath = join(tempDir, 'curl');
  writeFileSync(curlPath, '#!/bin/sh\nexec /usr/bin/curl --http1.1 "$@"\n');
  chmodSync(curlPath, 0o755);
  env.PATH = env.PATH ? `${tempDir}${delimiter}${env.PATH}` : tempDir;
}

const result = spawnSync(command, args, {
  env,
  stdio: 'inherit',
});

if (tempDir) {
  rmSync(tempDir, { force: true, recursive: true });
}

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { expect, it } from 'vitest';

it('enables the compile cache before evaluating the CLI module', () => {
  const root = mkdtempSync(join(tmpdir(), 'vp-bootstrap-'));
  try {
    mkdirSync(join(root, 'bin'));
    mkdirSync(join(root, 'dist'));
    writeFileSync(join(root, 'package.json'), JSON.stringify({ type: 'module' }));
    copyFileSync(new URL('../../bin/vp', import.meta.url), join(root, 'bin', 'vp'));
    writeFileSync(
      join(root, 'dist', 'bin.js'),
      `import { getCompileCacheDir } from 'node:module';
console.log(JSON.stringify({ enabled: getCompileCacheDir() !== undefined, args: process.argv.slice(2) }));
process.exitCode = 17;
`,
    );
    const env = { ...process.env };
    delete env.NODE_COMPILE_CACHE;
    delete env.NODE_DISABLE_COMPILE_CACHE;
    delete env.NODE_OPTIONS;
    const result = spawnSync(
      process.execPath,
      [join(root, 'bin', 'vp'), 'check', 'file with spaces.ts'],
      {
        env,
        encoding: 'utf8',
        timeout: 10_000,
      },
    );
    expect(result.error).toBeUndefined();
    expect(result.status).toBe(17);
    expect(JSON.parse(result.stdout)).toEqual({
      enabled: true,
      args: ['check', 'file with spaces.ts'],
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

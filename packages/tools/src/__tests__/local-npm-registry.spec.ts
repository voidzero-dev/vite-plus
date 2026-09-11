import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, it } from 'vitest';

it('isolates and cleans the platform temp directory used by bunx on each registry run', () => {
  const script = fileURLToPath(new URL('../local-npm-registry.ts', import.meta.url));
  const probe = `
    const fs = require('node:fs');
    console.log(JSON.stringify({
      cache: process.env.BUN_INSTALL_CACHE_DIR,
      temporary: process.env.BUN_TMPDIR,
      platformTemporary: process.env.TMPDIR,
      exists: fs.existsSync(process.env.TMPDIR ?? ''),
    }));
  `;
  const caches = new Set<string>();
  for (let run = 0; run < 2; run++) {
    const output = execFileSync(process.execPath, [script, '--', process.execPath, '-e', probe], {
      cwd: path.dirname(script),
      encoding: 'utf8',
      timeout: 10_000,
      env: { ...process.env, SNAP_LOCAL_VP_PACKAGES_DIR: '' },
    });
    const result = JSON.parse(output);
    expect(result.platformTemporary).toBe(path.join(result.cache, '.tmp'));
    expect(result.temporary).toBe(result.platformTemporary);
    expect(result.exists).toBe(true);
    expect(existsSync(result.cache)).toBe(false);
    expect(caches.has(result.cache)).toBe(false);
    caches.add(result.cache);
  }
});

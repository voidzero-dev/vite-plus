import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, it } from 'vitest';

it('serves fixture versions and dist-tags without resolving them from the public registry', () => {
  const script = fileURLToPath(new URL('../local-npm-registry.ts', import.meta.url));
  const cwd = mkdtempSync(path.join(tmpdir(), 'vp-registry-versions-'));
  const names = ['degit', '@test/template'];
  const manifest = Object.fromEntries(
    names.map((name) => [
      name,
      {
        name,
        'dist-tags': { latest: '0.0.0' },
        versions: {
          '0.0.0': {
            name,
            version: '0.0.0',
            dist: { tarball: `{REGISTRY}/${name}/-/template.tgz` },
          },
        },
      },
    ]),
  );
  try {
    writeFileSync(path.join(cwd, 'mock-manifest.json'), JSON.stringify(manifest));
    const probe = `
      const assert = require('node:assert/strict');
      (async () => {
        const registry = process.env.NPM_CONFIG_REGISTRY;
        for (const name of ${JSON.stringify(names)}) {
          const packument = await (await fetch(registry + '/' + encodeURIComponent(name))).json();
          for (const version of ['latest', '0.0.0']) {
            const response = await fetch(registry + '/' + encodeURIComponent(name) + '/' + version);
            assert.equal(response.status, 200);
            const actual = await response.json();
            assert.deepEqual(actual, packument.versions['0.0.0']);
            assert.equal(actual.dist.tarball, registry + '/' + name + '/-/template.tgz');
          }
        }
      })().catch((error) => { console.error(error); process.exitCode = 1; });
    `;
    execFileSync(process.execPath, [script, '--', process.execPath, '-e', probe], {
      cwd,
      timeout: 10_000,
      env: { ...process.env, SNAP_LOCAL_VP_PACKAGES_DIR: '' },
    });
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
});

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

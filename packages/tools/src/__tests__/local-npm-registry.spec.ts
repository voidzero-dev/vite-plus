import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
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

it('serves shared package metadata, version endpoints, and exact tarballs without upstream requests', () => {
  const script = fileURLToPath(new URL('../local-npm-registry.ts', import.meta.url));
  const fixtures = fileURLToPath(
    new URL('../../../../crates/vp_cli_snapshots/tests/cli_snapshots/registry', import.meta.url),
  );
  const root = mkdtempSync(path.join(tmpdir(), 'vp-registry-fixtures-test-'));
  try {
    // An accidental fallback fails immediately instead of reaching npm.
    writeFileSync(path.join(root, '.npmrc'), 'registry=https://127.0.0.1:9\n');
    writeFileSync(
      path.join(root, 'mock-manifest.json'),
      JSON.stringify({
        'testnpm2/fixture-tag': { name: 'fixture override', version: '9.0.0' },
      }),
    );
    const log = path.join(root, 'requests.jsonl');
    const probe = `
      const { createHash } = require('node:crypto');
      (async () => {
        const registry = process.env.NPM_CONFIG_REGISTRY;
        const metadata = await (await fetch(registry + '/testnpm2')).json();
        const version = await (await fetch(registry + '/testnpm2/latest')).json();
        const bytes = Buffer.from(await (await fetch(version.dist.tarball)).arrayBuffer());
        const integrity = 'sha512-' + createHash('sha512').update(bytes).digest('base64');
        if (integrity !== version.dist.integrity) throw new Error('tarball integrity mismatch');
        if (version.version !== metadata['dist-tags'].latest) throw new Error('version mismatch');
        if (!version.dist.tarball.startsWith(registry + '/')) throw new Error('remote tarball');
        const override = await (await fetch(registry + '/testnpm2/fixture-tag')).json();
        if (override.name !== 'fixture override') throw new Error('fixture did not win');
        console.log(version.name + '@' + version.version);
      })().catch(error => { console.error(error); process.exitCode = 1; });
    `;
    const output = execFileSync(
      process.execPath,
      [
        script,
        '--snapshot-cache',
        path.join(root, 'cache'),
        '--snapshot-fixtures',
        fixtures,
        '--request-log',
        log,
        '--',
        process.execPath,
        '-e',
        probe,
      ],
      {
        cwd: root,
        encoding: 'utf8',
        timeout: 10_000,
        env: { ...process.env, SNAP_LOCAL_VP_PACKAGES_DIR: '' },
      },
    );
    expect(output.trim()).toBe('testnpm2@1.0.1');
    expect(
      readFileSync(log, 'utf8')
        .trim()
        .split('\n')
        .map((line) => JSON.parse(line).event),
    ).toEqual(['fixture', 'fixture', 'fixture', 'fixture']);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

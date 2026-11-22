import { execFile, execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

import { RegistryCache } from '../registry-cache.ts';

it('prewarms every generated archive once and picks up changed fixture versions', async () => {
  const script = fileURLToPath(new URL('../local-npm-registry.ts', import.meta.url));
  const root = mkdtempSync(path.join(tmpdir(), 'vp-prewarm-manifest-test-'));
  const requests: string[] = [];
  const server = createServer((req, res) => {
    requests.push(req.url!);
    res.end(Buffer.from([0, 255, 128]));
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  try {
    const address = server.address();
    if (!address || typeof address === 'string') {
      throw new Error('Missing registry address');
    }
    const registry = `http://127.0.0.1:${address.port}`;
    const manifest = path.join(root, 'prewarm.json');
    const cache = path.join(root, 'cache');
    const first = `${registry}/bun-1.0.0.tgz`;
    const second = `${registry}/bun-2.0.0.tgz`;
    const run = () =>
      promisify(execFile)(
        process.execPath,
        [script, '--snapshot-cache', cache, '--prewarm-manifest', manifest],
        { timeout: 10_000 },
      );
    writeFileSync(manifest, JSON.stringify([first, first]));
    await run();
    writeFileSync(manifest, JSON.stringify([first, second]));
    await run();
    expect(requests).toEqual(['/bun-1.0.0.tgz', '/bun-2.0.0.tgz']);
    for (const url of [first, second]) {
      expect((await new RegistryCache(cache).get(url, 'application/octet-stream')).body).toEqual(
        Buffer.from([0, 255, 128]),
      );
    }
    expect(requests).toHaveLength(2);
  } finally {
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    rmSync(root, { recursive: true, force: true });
  }
});

it.each([false, true])(
  'shares a registry with snapshot processes without changing their environment (packed: %s)',
  (packed) => {
    const script = fileURLToPath(new URL('../local-npm-registry.ts', import.meta.url));
    const fixtures = fileURLToPath(
      new URL('../../../../crates/vp_cli_snapshots/tests/cli_snapshots/registry', import.meta.url),
    );
    const root = mkdtempSync(path.join(tmpdir(), 'vp-registry-runner-test-'));
    try {
      writeFileSync(path.join(root, '.npmrc'), 'registry=https://127.0.0.1:9\n');
      // The runner prepares packed archives after the wrapper starts. A request
      // for the packed registry must load them before checking local versions.
      const probe = `
        const { cpSync, mkdirSync } = require('node:fs');
        const packed = ${packed};
        const env = JSON.parse(process.env[packed
          ? 'VP_SNAP_PACKED_REGISTRY_ENV' : 'VP_SNAP_SHARED_REGISTRY_ENV']);
        if (packed) {
          mkdirSync('packages');
          cpSync(${JSON.stringify(path.join(fixtures, 'tarballs/is-number-7.0.0.tgz'))},
            'packages/is-number-7.0.0.tgz');
        }
        (async () => {
          const registry = env.NPM_CONFIG_REGISTRY;
          const name = packed ? 'is-number' : 'vp-test-echo';
          const responses = await Promise.all([1, 2].map(async () => {
            const metadata = await (await fetch(registry + '/' + name + '/latest')).json();
            const bytes = await (await fetch(metadata.dist.tarball)).arrayBuffer();
            if (!bytes.byteLength) throw new Error('empty tarball');
            return metadata.version;
          }));
          console.log(JSON.stringify({ env, responses,
            registry: process.env.NPM_CONFIG_REGISTRY, cache: process.env.YARN_GLOBAL_FOLDER }));
          process.exitCode = 7;
        })().catch(error => { console.error(error); process.exitCode = 1; });
      `;
      const result = spawnSync(
        process.execPath,
        [
          script,
          '--snapshot-runner',
          '--live',
          '--snapshot-fixtures',
          fixtures,
          ...(packed ? ['--packages-dir', path.join(root, 'packages')] : []),
          '--',
          process.execPath,
          '-e',
          probe,
        ],
        {
          cwd: root,
          encoding: 'utf8',
          timeout: 10_000,
          env: {
            ...process.env,
            SNAP_LOCAL_VP_PACKAGES_DIR: '',
            NPM_CONFIG_REGISTRY: 'https://127.0.0.1:9',
            YARN_GLOBAL_FOLDER: 'unchanged',
          },
        },
      );
      expect(result.stderr).toBe('');
      expect(result.status).toBe(7);
      const output = JSON.parse(result.stdout);
      expect(output.responses).toEqual(Array(2).fill(packed ? '7.0.0' : '1.0.0'));
      expect(output.registry).toBe('https://127.0.0.1:9');
      expect(output.cache).toBe('unchanged');
      expect(existsSync(output.env.YARN_GLOBAL_FOLDER)).toBe(false);
      expect(existsSync(output.env.BUN_INSTALL_CACHE_DIR)).toBe(false);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  },
);

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

it.each([false, true])(
  'serves fixture metadata and tarballs without upstream requests (live: %s)',
  (live) => {
    const script = fileURLToPath(new URL('../local-npm-registry.ts', import.meta.url));
    const fixtures = fileURLToPath(
      new URL('../../../../crates/vp_cli_snapshots/tests/cli_snapshots/registry', import.meta.url),
    );
    const root = mkdtempSync(path.join(tmpdir(), 'vp-registry-fixtures-test-'));
    const packages = live
      ? ['vp-test-echo', 'vp-test-upper']
      : ['testnpm2', 'vp-test-echo', 'vp-test-upper', 'is-number', 'yocto-queue'];
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
      const assert = require('node:assert/strict');
      const { createHash } = require('node:crypto');
      (async () => {
        const registry = process.env.NPM_CONFIG_REGISTRY;
        const selected = {
          'is-number': ['6.0.0', '7.0.0'],
          'yocto-queue': ['0.1.0', '1.0.0', '1.2.2'],
        };
        for (const name of ${JSON.stringify(packages)}) {
          const metadata = await (await fetch(registry + '/' + name)).json();
          const version = await (await fetch(registry + '/' + name + '/latest')).json();
          if (version.version !== metadata['dist-tags'].latest) throw new Error('version mismatch');
          if (selected[name]) assert.deepEqual(Object.keys(metadata.versions), selected[name]);
          for (const recorded of Object.values(metadata.versions)) {
            if (!recorded.dist.tarball.startsWith(registry + '/')) throw new Error('remote tarball');
            const bytes = Buffer.from(await (await fetch(recorded.dist.tarball)).arrayBuffer());
            const integrity = 'sha512-' + createHash('sha512').update(bytes).digest('base64');
            if (integrity !== recorded.dist.integrity) throw new Error('tarball integrity mismatch');
          }
          console.log(version.name + '@' + version.version);
        }
        const override = await (await fetch(registry + '/testnpm2/fixture-tag')).json();
        if (override.name !== 'fixture override') throw new Error('fixture did not win');
      })().catch(error => { console.error(error); process.exitCode = 1; });
    `;
      const output = execFileSync(
        process.execPath,
        [
          script,
          ...(live ? ['--live'] : []),
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
      expect(output.trim().split(/\r?\n/)).toEqual(
        live
          ? ['vp-test-echo@1.0.0', 'vp-test-upper@1.0.0']
          : [
              'testnpm2@1.0.1',
              'vp-test-echo@1.0.0',
              'vp-test-upper@1.0.0',
              'is-number@7.0.0',
              'yocto-queue@1.2.2',
            ],
      );
      expect(
        readFileSync(log, 'utf8')
          .trim()
          .split('\n')
          .map((line) => JSON.parse(line).event),
      ).toEqual(Array(live ? 7 : 20).fill('fixture'));
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  },
);

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cleanupRustCaches, supersededRustCaches } from '../cleanup-rust-caches.mjs';

const now = Date.parse('2026-10-21T00:00:00Z');
const day = 24 * 60 * 60 * 1000;

function cache(id, overrides = {}) {
  return {
    id,
    ref: 'refs/heads/main',
    key: `v1-rust-test-Linux-x64-12345678-${id.toString(16).padStart(8, '0')}`,
    version: 'archive-version',
    created_at: new Date(now - (6 - id) * day).toISOString(),
    last_accessed_at: new Date(now - (6 - id) * day).toISOString(),
    size_in_bytes: 500 * 1024 ** 2,
    ...overrides,
  };
}

await test('keeps the newest generation and expires unused older generations', () => {
  assert.deepEqual(supersededRustCaches([cache(2), cache(1), cache(3)], now), [cache(2), cache(1)]);
});

await test('preserves recently used or created entries, including the 48-hour boundary', () => {
  for (const field of ['last_accessed_at', 'created_at']) {
    for (const age of [0, 1, 2]) {
      const recent = cache(1, { [field]: new Date(now - age * day).toISOString() });
      assert.deepEqual(supersededRustCaches([recent, cache(5)], now), []);
    }
  }
});

await test('never treats different environments, jobs, architectures, or archive formats as replacements', () => {
  const first = cache(1);
  for (const overrides of [
    { key: cache(2).key.replace('12345678', '87654321') },
    { key: cache(2).key.replace('-test-', '-lint-') },
    { key: cache(2).key.replace('Linux-x64', 'Darwin-arm64') },
    { version: 'different-archive-version' },
  ]) {
    assert.deepEqual(supersededRustCaches([first, cache(2, overrides)], now), []);
  }
});

await test('ignores PR caches, other prefixes, invalid dates, and equal creation times', () => {
  for (const overrides of [
    { ref: 'refs/pull/12/merge' },
    { key: 'v1-rust-unrecognized-key' },
    { key: cache(1).key.replace('v1-rust', 'v2-rust') },
    { created_at: 'invalid' },
    { last_accessed_at: 'invalid' },
    { version: '' },
    { created_at: cache(3).created_at },
  ]) {
    assert.deepEqual(supersededRustCaches([cache(1, overrides), cache(3)], now), []);
  }
});

function fixture(snapshots, status) {
  const calls = [];
  let reads = 0;
  return {
    calls,
    context: { ref: 'refs/heads/main', repo: { owner: 'voidzero-dev', repo: 'vite-plus' } },
    core: { info() {} },
    github: {
      async paginate(method, params) {
        calls.push({ type: 'list', params });
        return snapshots[Math.min(reads++, snapshots.length - 1)];
      },
      rest: {
        actions: {
          getActionsCacheList() {},
          async deleteActionsCacheById(params) {
            calls.push({ type: 'delete', params });
            if (status) {
              throw Object.assign(new Error('API error'), { status });
            }
          },
        },
      },
    },
  };
}

await test('defaults to a dry run with no writes and reports eligible bytes', async () => {
  const f = fixture([[cache(1), cache(3)]]);
  const result = await cleanupRustCaches(f);
  assert.deepEqual(result, { count: 1, bytes: cache(1).size_in_bytes });
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].params.ref, 'refs/heads/main');
  assert.equal(f.calls[0].params.per_page, 100);
});

await test('rechecks candidates and deletes by ID', async () => {
  const f = fixture([[cache(1), cache(3)]]);
  await cleanupRustCaches(f, { dryRun: false });
  assert.deepEqual(
    f.calls.map((call) => call.type),
    ['list', 'list', 'delete'],
  );
  assert.equal(f.calls[1].params.key, 'v1-rust-test-Linux-x64-12345678-');
  assert.equal(f.calls[2].params.cache_id, 1);
});

await test('skips entries used since the snapshot or whose replacement disappeared', async () => {
  for (const second of [
    [cache(1, { last_accessed_at: new Date().toISOString() }), cache(3)],
    [cache(1)],
    [cache(3)],
  ]) {
    const f = fixture([[cache(1), cache(3)], second]);
    assert.deepEqual(await cleanupRustCaches(f, { dryRun: false }), { count: 0, bytes: 0 });
    assert.ok(f.calls.every((call) => call.type === 'list'));
  }
});

await test('tolerates concurrent eviction but reports other API failures', async () => {
  assert.deepEqual(
    await cleanupRustCaches(fixture([[cache(1), cache(3)]], 404), { dryRun: false }),
    { count: 0, bytes: 0 },
  );
  await assert.rejects(
    cleanupRustCaches(fixture([[cache(1), cache(3)]], 403), { dryRun: false }),
    /API error/,
  );
});

await test('refuses a non-main run before making API requests', async () => {
  const f = fixture([]);
  f.context.ref = 'refs/heads/feature';
  await assert.rejects(cleanupRustCaches(f, { dryRun: false }), /must run on main/);
  assert.deepEqual(f.calls, []);
});

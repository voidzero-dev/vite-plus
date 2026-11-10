import { mkdtemp, rm } from 'node:fs/promises';
import { createServer, type Server, type RequestListener } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { gzipSync } from 'node:zlib';

import { afterEach, expect, it } from 'vitest';

import { RegistryCache, rewriteTarballs, upstreamUrl } from '../registry-cache.ts';

const directories: string[] = [];
const servers: Server[] = [];

afterEach(async () => {
  for (const server of servers.splice(0)) {
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
  await Promise.all(directories.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
});

async function directory() {
  const dir = await mkdtemp(path.join(tmpdir(), 'vp-registry-cache-test-'));
  directories.push(dir);
  return dir;
}

async function upstream(handler: RequestListener) {
  const server = createServer(handler);
  servers.push(server);
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  if (!address || typeof address === 'string') {
    throw new Error('Missing test server address');
  }
  return `http://127.0.0.1:${address.port}`;
}

it('coalesces requests across registries and preserves binary response bytes', async () => {
  let requests = 0;
  const bytes = Buffer.from([0, 255, 10, 128, 31, 139]);
  const url = await upstream((_req, res) => {
    requests++;
    res.end(bytes);
  });
  const dir = await directory();
  const registries = [new RegistryCache(dir), new RegistryCache(dir)];
  const responses = await Promise.all(
    Array.from({ length: 12 }, (_, i) => registries[i % 2].get(url, 'application/octet-stream')),
  );
  expect(requests).toBe(1);
  for (const response of responses) {
    expect(response.body).toEqual(bytes);
  }
  expect((await new RegistryCache(dir).get(url, 'application/octet-stream')).body).toEqual(bytes);
  expect(requests).toBe(1);
});

it('separates metadata representations and upstream registries', async () => {
  let requests = 0;
  const handler: RequestListener = (req, res) => {
    requests++;
    res.end(req.headers.accept);
  };
  const first = await upstream(handler);
  const second = await upstream(handler);
  const cache = new RegistryCache(await directory());
  expect((await cache.get(first, 'application/json')).body.toString()).toBe('application/json');
  expect((await cache.get(first, 'application/vnd.npm.install-v1+json')).body.toString()).toBe(
    'application/vnd.npm.install-v1+json',
  );
  await cache.get(second, 'application/json');
  expect(requests).toBe(3);
});

it('does not retain failed responses or partial downloads', async () => {
  let requests = 0;
  const url = await upstream((_req, res) => {
    requests++;
    if (requests === 1) {
      res.writeHead(503);
      res.end('retry');
    } else if (requests === 2) {
      res.writeHead(200, { 'content-length': 100 });
      res.write('partial', () => res.destroy());
    } else {
      res.end('complete');
    }
  });
  const cache = new RegistryCache(await directory());
  expect((await cache.get(url, 'application/json')).status).toBe(503);
  await expect(cache.get(url, 'application/json')).rejects.toThrow();
  expect((await cache.get(url, 'application/json')).body.toString()).toBe('complete');
  await cache.get(url, 'application/json');
  expect(requests).toBe(3);
});

it('can bypass response storage for a live-network benchmark', async () => {
  let requests = 0;
  const url = await upstream((_req, res) => res.end(String(++requests)));
  const cache = new RegistryCache(await directory(), false);
  expect((await cache.get(url, 'application/json')).body.toString()).toBe('1');
  expect((await cache.get(url, 'application/json')).body.toString()).toBe('2');
});

it('prepares an archive once for eight workers with shorter request budgets', async () => {
  let requests = 0;
  const bytes = Buffer.from([0, 255, 10, 128, 31, 139]);
  const url = `${await upstream((_req, res) => {
    requests++;
    setTimeout(() => res.end(bytes), 150);
  })}/tool.tgz`;
  const dir = await directory();
  await new RegistryCache(dir).prewarm(url);
  const responses = await Promise.all(
    Array.from({ length: 8 }, () =>
      new RegistryCache(dir, true, undefined, 50).get(url, 'application/json'),
    ),
  );
  expect(requests).toBe(1);
  for (const response of responses) {
    expect(response.body).toEqual(bytes);
  }
});

it('retries a failed preparation without caching the failed response', async () => {
  let requests = 0;
  const url = await upstream((_req, res) => {
    if (++requests === 1) {
      res.writeHead(503);
      res.end('retry');
    } else {
      res.end('archive');
    }
  });
  const cache = new RegistryCache(await directory());
  await cache.prewarm(url);
  expect((await cache.get(url, 'application/octet-stream')).body.toString()).toBe('archive');
  expect(requests).toBe(2);
});

it('bounds an unfinished download and releases its lock for another registry', async () => {
  let complete = false;
  const url = await upstream((_req, res) => {
    if (complete) {
      res.end('complete');
      return;
    }
    const timer = setInterval(() => res.write('partial'), 5);
    res.on('close', () => clearInterval(timer));
  });
  const dir = await directory();
  const cache = new RegistryCache(dir, true, undefined, 100);
  await expect(cache.get(url, 'application/octet-stream')).rejects.toMatchObject({
    name: 'AbortError',
  });
  complete = true;
  const next = new RegistryCache(dir);
  expect((await next.get(url, 'application/octet-stream')).body.toString()).toBe('complete');
});

it.each(['headers', 'body'])(
  'releases a download stalled before %s for a client retry',
  async (stage) => {
    let complete = false;
    const url = await upstream((_req, res) => {
      if (complete) {
        res.end('complete');
      } else if (stage === 'body') {
        res.writeHead(200);
        res.flushHeaders();
      }
    });
    const dir = await directory();
    const cache = new RegistryCache(dir, true, undefined, 60_000, 100);
    await expect(cache.get(url, 'application/octet-stream')).rejects.toMatchObject({
      name: 'AbortError',
      cause: { message: 'Registry request stalled' },
    });
    complete = true;
    const next = new RegistryCache(dir);
    expect((await next.get(url, 'application/octet-stream')).body.toString()).toBe('complete');
  },
);

it('routes tarballs through the case registry without changing unrelated metadata', () => {
  const registry = 'http://127.0.0.1:1234';
  const cdn = 'https://cdn.example.test/package.tgz';
  const result = rewriteTarballs(
    {
      homepage: cdn,
      versions: {
        '1.0.0': {
          dist: {
            tarball: 'https://registry.npmjs.org/example/-/example-1.0.0.tgz',
            integrity: 'test',
          },
        },
        '2.0.0': { dist: { tarball: cdn } },
        '3.0.0': { dist: { tarball: `${registry}/local.tgz` } },
      },
    },
    registry,
  ) as {
    homepage: string;
    versions: Record<string, { dist: { tarball: string; integrity?: string } }>;
  };
  expect(result.homepage).toBe(cdn);
  expect(result.versions['1.0.0'].dist).toEqual({
    tarball: `${registry}/example/-/example-1.0.0.tgz`,
    integrity: 'test',
  });
  expect(
    upstreamUrl(
      new URL(result.versions['2.0.0'].dist.tarball).pathname,
      'https://registry.npmjs.org',
    ),
  ).toBe(cdn);
  expect(result.versions['3.0.0'].dist.tarball).toBe(`${registry}/local.tgz`);
});

it('decodes compressed metadata before storing it for other package managers', async () => {
  const metadata = JSON.stringify({ name: 'example', versions: {} });
  let requests = 0;
  const url = await upstream((_req, res) => {
    requests++;
    res.writeHead(200, { 'content-type': 'application/json', 'content-encoding': 'gzip' });
    res.end(gzipSync(metadata));
  });
  const cache = new RegistryCache(await directory());
  expect((await cache.get(url, 'application/json')).body.toString()).toBe(metadata);
  expect((await cache.get(url, 'application/json')).body.toString()).toBe(metadata);
  expect(requests).toBe(1);
});

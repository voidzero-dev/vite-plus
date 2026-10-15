import { once } from 'node:events';
import { createServer, get, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';

import { afterEach, beforeEach, expect, it, vi } from 'vitest';

import { setRegistryRequestTimeout } from '../registry-request.ts';

let server: Server;
let url: string;

beforeEach(async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  server = createServer();
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  url = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});

afterEach(async () => {
  vi.useRealTimers();
  const closed = once(server, 'close');
  server.close();
  server.closeAllConnections();
  await closed;
});

it('aborts an upstream that never sends response headers', async () => {
  const received = once(server, 'request');
  const request = get(url);
  const failed = once(request, 'error');
  setRegistryRequestTimeout(request, 100);
  await received;

  vi.advanceTimersByTime(100);
  const [error] = await failed;
  expect(error.message).toBe('Registry upstream request timed out after 100ms');
  expect(request.destroyed).toBe(true);
});

it('aborts an upstream while DNS lookup is stalled', async () => {
  const request = get('http://registry.invalid', { lookup: () => {} });
  const failed = once(request, 'error');
  setRegistryRequestTimeout(request, 100);

  vi.advanceTimersByTime(100);
  const [error] = await failed;
  expect(error.message).toContain('timed out');
  expect(request.destroyed).toBe(true);
});

it('aborts a response body that stops sending data', async () => {
  vi.useRealTimers();
  server.on('request', (_req, res) => {
    res.writeHead(200, { 'content-type': 'application/json' });
    res.write('{');
  });
  const request = get(url);
  const response = once(request, 'response');
  const failed = once(request, 'error');
  setRegistryRequestTimeout(request, 100);
  const [upstream] = await response;
  const bodyFailed = once(upstream, 'error');
  upstream.resume();

  const [error] = await failed;
  expect(error.message).toContain('timed out');
  await bodyFailed;
  expect(upstream.complete).toBe(false);
});

it('allows a response body to exceed the timeout while data keeps arriving', async () => {
  vi.useRealTimers();
  server.on('request', async (_req, res) => {
    res.write('a');
    for (let chunk = 0; chunk < 6; chunk++) {
      await delay(250);
      if (res.destroyed) {
        return;
      }
      res.write('b');
    }
    res.end();
  });
  const request = get(url);
  const failed = vi.fn();
  request.on('error', failed);
  setRegistryRequestTimeout(request, 1000);
  const [upstream] = await once(request, 'response');
  const chunks: Buffer[] = [];
  for await (const chunk of upstream) {
    chunks.push(chunk);
  }

  expect(Buffer.concat(chunks).toString()).toBe('abbbbbb');
  expect(upstream.complete).toBe(true);
  expect(failed).not.toHaveBeenCalled();
});

it('clears the deadline when a response finishes', async () => {
  server.on('request', (_req, res) => res.end('ok'));
  const request = get(url, (response) => response.resume());
  const closed = once(request, 'close');
  setRegistryRequestTimeout(request, 100);
  await closed;

  expect(vi.getTimerCount()).toBe(0);
  vi.advanceTimersByTime(100);
  expect(request.errored).toBeNull();
});

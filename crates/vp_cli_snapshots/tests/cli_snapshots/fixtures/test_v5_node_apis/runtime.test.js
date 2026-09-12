import { test, expect, vi, recordArtifact } from 'vite-plus/test';
import { writeFileSync } from 'node:fs';

test('one-based worker identifiers', () => {
  expect(Number(process.env.VITEST_POOL_ID)).toBeGreaterThanOrEqual(1);
  expect(Number(process.env.VITEST_WORKER_ID)).toBeGreaterThanOrEqual(1);
});

test('class mocks preserve their implementation prototype', () => {
  class Service { value() { return 42; } }
  const MockService = vi.fn(Service);
  const instance = new MockService();
  expect(instance).toBeInstanceOf(Service);
  expect(instance.value()).toBe(42);
});

test('annotations and custom artifacts', async ({ annotate, task }) => {
  writeFileSync('attachment.txt', 'v5 attachment content');
  await annotate('plain-text attachment', {
    path: 'attachment.txt',
    contentType: 'text/plain',
  });
  await recordArtifact(task, { type: 'fixture:identity', value: 42 });
});

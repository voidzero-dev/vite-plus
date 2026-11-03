import { expect, test } from 'vite-plus/test';

test('bench is a test-context fixture', async ({ bench }) => {
  let calls = 0;
  const result = await bench('sum', () => { calls++; return 20 + 22; }).run({
    iterations: 1, time: 1, warmup: false,
  });
  expect(calls).toBeGreaterThan(0);
  expect(result.name).toBe('sum');
});

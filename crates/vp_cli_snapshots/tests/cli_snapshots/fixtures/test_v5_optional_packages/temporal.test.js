import { afterEach, expect, test, vi } from 'vite-plus/test';

afterEach(() => vi.useRealTimers());

test('new defaults mock Temporal with fake timers', () => {
  const now = Temporal.Now.instant;
  vi.useFakeTimers();
  vi.setSystemTime(1234);
  expect(Temporal.Now.instant).not.toBe(now);
  expect(Temporal.Now.instant().epochMilliseconds).toBe(1234);
});

test('migrated toNotFake preserves Temporal while fake timers run', () => {
  const now = Temporal.Now.instant;
  vi.useFakeTimers({ toNotFake: ['Temporal'] });
  vi.setSystemTime(1234);
  expect(Temporal.Now.instant).toBe(now);
  // The polyfill can still read mocked Date internally; toNotFake preserves
  // its implementation, not every transitive source of wall-clock time.
});

test('setSystemTime also mocks Temporal without fake timers', () => {
  vi.setSystemTime(2345);
  expect(vi.isFakeTimers()).toBe(false);
  expect(Temporal.Now.instant().epochMilliseconds).toBe(2345);
});

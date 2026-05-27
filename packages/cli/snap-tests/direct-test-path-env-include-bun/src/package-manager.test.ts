import { execFileSync } from 'node:child_process';

import { expect, test } from '@voidzero-dev/vite-plus-test';

test('direct test command exposes the configured package manager on PATH', () => {
  const version = execFileSync('bun', ['--version'], { encoding: 'utf8' }).trim();
  expect(version).toBe('1.3.11');
});

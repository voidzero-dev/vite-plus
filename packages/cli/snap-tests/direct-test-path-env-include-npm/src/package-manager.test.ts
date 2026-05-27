import { execFileSync } from 'node:child_process';

import { expect, test } from '@voidzero-dev/vite-plus-test';

test('direct test command exposes the configured package manager on PATH', () => {
  const version = execFileSync('npm', ['--version'], { encoding: 'utf8' }).trim();
  expect(version).toBe('10.9.4');
});

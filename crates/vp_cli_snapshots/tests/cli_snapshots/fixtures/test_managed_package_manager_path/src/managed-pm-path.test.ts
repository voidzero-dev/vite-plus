import { execFileSync } from 'node:child_process';

import { expect, test } from 'vite-plus/test';

test('direct test command exposes the configured package manager on PATH', () => {
  const version = execFileSync('pnpm', ['--version'], { encoding: 'utf8' }).trim();
  expect(version).toBe('11.2.2');
});

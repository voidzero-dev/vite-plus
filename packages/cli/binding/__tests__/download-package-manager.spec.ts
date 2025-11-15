import { expect, test } from 'vitest';

import { downloadPackageManager } from '../index.js';

test('should download package manager successfully', async () => {
  const result = await downloadPackageManager({
    name: 'pnpm',
    version: 'latest',
  });
  expect(result.name).toBe('pnpm');
  expect(result.packageName).toBe('pnpm');
  expect(result.version).toMatch(/^\d+\.\d+\.\d+$/);
  expect(result.installDir).toBeTruthy();
  expect(result.binPrefix).toMatch(/bin$/);
});

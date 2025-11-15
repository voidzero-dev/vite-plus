import { tmpdir } from 'node:os';
import path from 'node:path';

import { expect, test } from 'vitest';

import { detectWorkspace } from '../index.js';

const fixtures = path.join(import.meta.dirname, 'fixtures');

test('should detect pnpm monorepo workspace successfully', async () => {
  const cwd = path.join(fixtures, 'pnpm-monorepo');
  const result = await detectWorkspace(cwd);
  expect(result.packageManagerName).toBe('pnpm');
  expect(result.packageManagerVersion).toBe('10.19.0');
  expect(result.isMonorepo).toBe(true);
  expect(result.root).toBe(cwd);

  // detect from sub directory
  const subCwd = path.join(cwd, 'packages', 'sub-package');
  const subResult = await detectWorkspace(subCwd);
  expect(subResult.packageManagerName).toBe('pnpm');
  expect(subResult.packageManagerVersion).toBe('10.19.0');
  expect(subResult.isMonorepo).toBe(true);
  expect(subResult.root).toBe(cwd);
});

test('should detect npm monorepo workspace successfully', async () => {
  const cwd = path.join(fixtures, 'npm-monorepo');
  const result = await detectWorkspace(cwd);
  expect(result.packageManagerName).toBe('npm');
  expect(result.packageManagerVersion).toBe('10.19.0');
  expect(result.isMonorepo).toBe(true);
  expect(result.root).toBe(cwd);
});

// FIXME: currently it will always find vite-plus, there is a problem here
test.skip('should detect npm project successfully', async () => {
  const cwd = path.join(fixtures, 'npm-project');
  const result = await detectWorkspace(cwd);
  expect(result.packageManagerName).toBe('npm');
  expect(result.packageManagerVersion).toBe('10.19.0');
  expect(result.isMonorepo).toBe(false);
  expect(result.root).toBe(cwd);
});

test('should detect workspace failed with not exists directory', async () => {
  const result = await detectWorkspace(path.join(tmpdir(), 'not-exists'));
  expect(result.packageManagerName).toBeUndefined();
  expect(result.packageManagerVersion).toBeUndefined();
  expect(result.isMonorepo).toBe(false);
  expect(result.root).toBeUndefined();
});

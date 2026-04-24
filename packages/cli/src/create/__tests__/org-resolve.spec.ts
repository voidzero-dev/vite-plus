import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { getConfiguredDefaultTemplate } from '../org-resolve.js';

describe('getConfiguredDefaultTemplate', () => {
  let repoRoot: string;

  beforeEach(() => {
    repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-org-resolve-'));
  });

  afterEach(() => {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  });

  function writeRepoConfig(defaultTemplate: string): void {
    fs.mkdirSync(path.join(repoRoot, '.git'), { recursive: true });
    // Plain object export instead of `defineConfig` — the test only
    // needs the shape to be readable, and dropping the `vite-plus`
    // import avoids noisy module-not-found warnings from vite's loader.
    fs.writeFileSync(
      path.join(repoRoot, 'vite.config.ts'),
      `export default { create: { defaultTemplate: '${defaultTemplate}' } };\n`,
    );
    fs.writeFileSync(path.join(repoRoot, 'package.json'), '{"name":"fixture"}');
  }

  it('reads the defaultTemplate from the repo root when invoked at the root', async () => {
    writeRepoConfig('@your-org');
    expect(await getConfiguredDefaultTemplate(repoRoot)).toBe('@your-org');
  });

  it('walks up from a subdirectory to find the repo root config', async () => {
    writeRepoConfig('@your-org');
    const deep = path.join(repoRoot, 'apps', 'web', 'src');
    fs.mkdirSync(deep, { recursive: true });
    expect(await getConfiguredDefaultTemplate(deep)).toBe('@your-org');
  });

  it('returns undefined when no vite.config exists anywhere up the tree', async () => {
    const deep = path.join(repoRoot, 'nested');
    fs.mkdirSync(deep, { recursive: true });
    expect(await getConfiguredDefaultTemplate(deep)).toBeUndefined();
  });

  it('returns undefined when vite.config has no create.defaultTemplate', async () => {
    fs.mkdirSync(path.join(repoRoot, '.git'), { recursive: true });
    fs.writeFileSync(path.join(repoRoot, 'vite.config.ts'), 'export default {};\n');
    fs.writeFileSync(path.join(repoRoot, 'package.json'), '{"name":"fixture"}');
    expect(await getConfiguredDefaultTemplate(repoRoot)).toBeUndefined();
  });
});

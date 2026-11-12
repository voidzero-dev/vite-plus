import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import { rewriteMonorepoProject } from '../migrator.ts';

let project: string;

beforeEach(() => {
  project = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-workspace-pack-'));
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({ name: 'library', devDependencies: { 'vite-plus': 'latest' } }),
  );
});

afterEach(() => {
  fs.rmSync(project, { recursive: true, force: true });
});

it('migrates legacy pack options when integrating a new workspace library', () => {
  const configPath = path.join(project, 'vite.config.ts');
  fs.writeFileSync(
    configPath,
    `import { defineConfig } from 'vite-plus';
export default defineConfig({ pack: { dts: { tsgo: true }, exports: true } });
`,
  );

  rewriteMonorepoProject(project, PackageManager.pnpm, true, true);

  const migrated = fs.readFileSync(configPath, 'utf8');
  expect(migrated).toContain("generator: 'tsgo'");
  expect(migrated).not.toContain('tsgo: true');
  expect(migrated).toContain('exports: true');

  rewriteMonorepoProject(project, PackageManager.pnpm, true, true);
  expect(fs.readFileSync(configPath, 'utf8')).toBe(migrated);
});

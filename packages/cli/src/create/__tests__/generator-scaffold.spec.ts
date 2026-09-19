import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { PackageManager, type WorkspaceInfo } from '../../types/index.ts';
import { readJsonFile } from '../../utils/json.ts';
import { templatesDir } from '../../utils/path.ts';
import { readYamlFile } from '../../utils/yaml.ts';
import { executeGeneratorScaffold } from '../templates/generator.ts';
import { BuiltinTemplate, TemplateType } from '../templates/types.ts';

let rootDir: string;
afterEach(() => {
  if (rootDir) {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});

async function scaffold(packageManager: PackageManager, version: string) {
  rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-generator-scaffold-'));
  const workspace: WorkspaceInfo = {
    rootDir,
    isMonorepo: true,
    monorepoScope: '',
    workspacePatterns: ['tools/*'],
    parentDirs: ['tools'],
    packageManager,
    packageManagerVersion: version,
    downloadPackageManager: {
      name: packageManager,
      packageName: packageManager,
      version,
      installDir: '',
      binPrefix: '',
    },
    packages: [],
  };
  await executeGeneratorScaffold(
    workspace,
    {
      command: BuiltinTemplate.generator,
      type: TemplateType.builtin,
      packageName: 'my-generator',
      targetDir: 'tools/my-generator',
      interactive: false,
      args: [],
      envs: {},
    },
    { silent: true },
  );
  return readJsonFile(path.join(rootDir, 'tools/my-generator/package.json'));
}

describe('generator scaffold dependencies', () => {
  it.each([
    [PackageManager.npm, '12.0.2'],
    [PackageManager.yarn, '4.9.0'],
    [PackageManager.pnpm, '9.4.0'],
  ])('uses installable versions for %s %s without catalog support', async (manager, version) => {
    const pkg = await scaffold(manager, version);
    const { catalog } = readYamlFile(path.join(templatesDir, 'monorepo/pnpm-workspace.yaml'));
    expect(pkg.devDependencies).toEqual(catalog);
    expect(pkg.name).toBe('my-generator');
    expect(pkg.dependencies).toEqual({ bingo: '^0.9.3', zod: '^3.25.76' });
  });

  it.each([
    [PackageManager.pnpm, '10.0.0'],
    [PackageManager.yarn, '4.10.0'],
    [PackageManager.bun, '1.3.0'],
  ])('preserves workspace catalogs for %s %s', async (manager, version) => {
    const pkg = await scaffold(manager, version);
    expect(pkg.devDependencies).toEqual({ '@types/node': 'catalog:', typescript: 'catalog:' });
  });
});

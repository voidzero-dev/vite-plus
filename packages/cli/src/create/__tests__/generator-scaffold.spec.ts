import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';
import { stringify } from 'yaml';

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

async function scaffold(
  packageManager: PackageManager,
  version: string,
  catalogs: {
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
  } = {},
) {
  rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-generator-scaffold-'));
  fs.writeFileSync(
    path.join(rootDir, 'package.json'),
    JSON.stringify({
      private: true,
      workspaces: ['tools/*'],
      ...(packageManager === PackageManager.bun ? catalogs : {}),
    }),
  );
  if (packageManager === PackageManager.pnpm) {
    fs.writeFileSync(
      path.join(rootDir, 'pnpm-workspace.yaml'),
      stringify({ packages: ['tools/*'], ...catalogs }),
    );
  } else if (packageManager === PackageManager.yarn) {
    fs.writeFileSync(path.join(rootDir, '.yarnrc.yml'), stringify(catalogs));
  }
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
    [PackageManager.pnpm, '10.0.0'],
    [PackageManager.yarn, '4.10.0'],
    [PackageManager.bun, '1.3.0'],
  ])('uses installable versions for %s %s without catalog entries', async (manager, version) => {
    const pkg = await scaffold(manager, version);
    const { catalog } = readYamlFile(path.join(templatesDir, 'monorepo/pnpm-workspace.yaml'));
    expect(pkg.devDependencies).toEqual(catalog);
    expect(pkg.name).toBe('my-generator');
    const templatePkg = readJsonFile(path.join(templatesDir, 'generator/package.json'));
    expect(pkg.dependencies).toEqual(templatePkg.dependencies);
  });

  it.each([
    [PackageManager.pnpm, '10.0.0'],
    [PackageManager.yarn, '4.10.0'],
    [PackageManager.bun, '1.3.0'],
  ])('preserves workspace catalogs for %s %s', async (manager, version) => {
    const pkg = await scaffold(manager, version, {
      catalog: { '@types/node': '^22', typescript: '^5' },
    });
    expect(pkg.devDependencies).toEqual({ '@types/node': 'catalog:', typescript: 'catalog:' });
  });

  it.each([
    [PackageManager.pnpm, '10.0.0'],
    [PackageManager.yarn, '4.10.0'],
    [PackageManager.bun, '1.3.0'],
  ])('fills only missing catalog dependencies for %s %s', async (manager, version) => {
    const pkg = await scaffold(manager, version, { catalog: { typescript: '^5' } });
    const { catalog } = readYamlFile(path.join(templatesDir, 'monorepo/pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
    };
    expect(pkg.devDependencies).toEqual({
      '@types/node': catalog['@types/node'],
      typescript: 'catalog:',
    });
  });

  it.each([
    [PackageManager.pnpm, '10.0.0'],
    [PackageManager.yarn, '4.10.0'],
    [PackageManager.bun, '1.3.0'],
  ])('resolves catalogs.default for %s %s', async (manager, version) => {
    const pkg = await scaffold(manager, version, {
      catalogs: { default: { '@types/node': '^22', typescript: '^5' } },
    });
    expect(pkg.devDependencies).toEqual({ '@types/node': 'catalog:', typescript: 'catalog:' });
  });
});

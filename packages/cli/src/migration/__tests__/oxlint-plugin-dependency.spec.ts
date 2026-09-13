import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  collectOxlintOwnerDirs,
  dropDeadOxlintPluginsDependency,
  packageOwnsOxlintApi,
  rewritePackageJson,
  sourceTreeReferencesOxlintPluginsPackage,
  usesVitestBrowserMode,
} from '../migrator.ts';

describe('Oxlint plugin dependency cleanup', () => {
  let projectPath: string;

  beforeEach(() => {
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxlint-plugin-dependency-'));
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
  });

  it.each([`node -e "require('@oxlint/plugins')"`, `node -e "import('@oxlint/plugins')"`])(
    'retains a dependency used by the inline script %s',
    (script) => {
      const pkg = {
        scripts: { 'check-plugin': script },
        devDependencies: { '@oxlint/plugins': '^1.79.0' },
      };
      rewritePackageJson(pkg, PackageManager.pnpm);
      const packageJsonPath = path.join(projectPath, 'package.json');
      fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));

      expect(pkg.scripts['check-plugin']).toBe(script);
      expect(sourceTreeReferencesOxlintPluginsPackage(projectPath)).toBe(true);
      dropDeadOxlintPluginsDependency(projectPath);

      expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
    },
  );

  it('retains a dependency used by a nested non-workspace package script', () => {
    const pkg = { devDependencies: { '@oxlint/plugins': '^1.79.0' } };
    const packageJsonPath = path.join(projectPath, 'package.json');
    fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
    const examplePath = path.join(projectPath, 'example');
    fs.mkdirSync(examplePath);
    fs.writeFileSync(
      path.join(examplePath, 'package.json'),
      JSON.stringify({ scripts: { 'check-plugin': `node -e "require('@oxlint/plugins')"` } }),
    );

    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
  });

  it('ignores dependency declarations and descriptive metadata when scripts use vite-plus', () => {
    const packageJsonPath = path.join(projectPath, 'package.json');
    const metadata = {
      description: 'Previously used @oxlint/plugins',
      scripts: { 'check-plugin': `node -e "require('vite-plus/lint/plugins')"` },
    };
    fs.writeFileSync(
      packageJsonPath,
      JSON.stringify({ ...metadata, devDependencies: { '@oxlint/plugins': '^1.79.0' } }),
    );

    expect(sourceTreeReferencesOxlintPluginsPackage(projectPath)).toBe(false);
    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual({
      ...metadata,
      devDependencies: {},
    });
  });

  it.each(['devDependencies', 'optionalDependencies'] as const)(
    'retains an import alias target in %s',
    (field) => {
      const pkg = {
        imports: { '#plugin-api': '@oxlint/plugins' },
        [field]: { '@oxlint/plugins': '^1.79.0' },
      };
      const packageJsonPath = path.join(projectPath, 'package.json');
      fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
      fs.writeFileSync(
        path.join(projectPath, 'plugin.js'),
        `import { defineRule } from '#plugin-api';`,
      );

      dropDeadOxlintPluginsDependency(projectPath);

      expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
    },
  );

  it('retains conditional aliases in nested non-workspace packages', () => {
    const pkg = { devDependencies: { '@oxlint/plugins': '^1.79.0' } };
    const packageJsonPath = path.join(projectPath, 'package.json');
    fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
    const examplePath = path.join(projectPath, 'example');
    fs.mkdirSync(examplePath);
    fs.writeFileSync(
      path.join(examplePath, 'package.json'),
      JSON.stringify({
        imports: {
          '#plugin-api': {
            node: { import: '@oxlint/plugins', require: '@oxlint/plugins' },
            default: './fallback.js',
          },
        },
      }),
    );
    fs.writeFileSync(
      path.join(examplePath, 'plugin.js'),
      `import { defineRule } from '#plugin-api';`,
    );

    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
  });

  it('removes unused dependencies when aliases already target vite-plus', () => {
    const packageJsonPath = path.join(projectPath, 'package.json');
    const imports = { '#plugin-api': 'vite-plus/lint/plugins' };
    fs.writeFileSync(
      packageJsonPath,
      JSON.stringify({
        imports,
        devDependencies: { '@oxlint/plugins': '^1.79.0', 'vite-plus': 'latest' },
      }),
    );
    fs.writeFileSync(
      path.join(projectPath, 'plugin.js'),
      `import { defineRule } from '#plugin-api';`,
    );

    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual({
      imports,
      devDependencies: { 'vite-plus': 'latest' },
    });
  });

  it.each(['dist', 'build', 'out', '.cache'])(
    'retains dependencies used by an ignored %s plugin',
    (directory) => {
      const pkg = { devDependencies: { '@oxlint/plugins': '^1.79.0' } };
      const packageJsonPath = path.join(projectPath, 'package.json');
      fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
      fs.writeFileSync(path.join(projectPath, '.gitignore'), `${directory}/\n`);
      fs.mkdirSync(path.join(projectPath, directory));
      fs.writeFileSync(
        path.join(projectPath, directory, 'plugin.cjs'),
        `const { defineRule } = require('@oxlint/plugins');`,
      );
      fs.writeFileSync(
        path.join(projectPath, directory, 'test.js'),
        `import { chromium } from '@vitest/browser-playwright';`,
      );

      expect(sourceTreeReferencesOxlintPluginsPackage(projectPath)).toBe(true);
      expect(usesVitestBrowserMode(projectPath)).toBe(false);
      dropDeadOxlintPluginsDependency(projectPath);

      expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
    },
  );

  it.each(['node_modules', '.git'])(
    'does not retain a dependency referenced only in %s',
    (directory) => {
      const packageJsonPath = path.join(projectPath, 'package.json');
      fs.writeFileSync(
        packageJsonPath,
        JSON.stringify({ devDependencies: { '@oxlint/plugins': '^1.79.0' } }),
      );
      fs.mkdirSync(path.join(projectPath, directory));
      fs.writeFileSync(
        path.join(projectPath, directory, 'plugin.cjs'),
        `require('@oxlint/plugins');`,
      );

      dropDeadOxlintPluginsDependency(projectPath);

      expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')).devDependencies).toEqual({});
    },
  );

  it('preserves an optional runtime API without introducing a development-only replacement', () => {
    const pkg = {
      name: 'oxlint-plugin-optional-example',
      optionalDependencies: { '@oxlint/plugins': '^1.79.0' },
      devDependencies: {},
    };
    const packageJsonPath = path.join(projectPath, 'package.json');
    fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));

    expect(collectOxlintOwnerDirs(projectPath)).toEqual([projectPath]);
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.optionalDependencies).toEqual({ '@oxlint/plugins': '^1.79.0' });
    expect(pkg.devDependencies).not.toHaveProperty('vite-plus');
    fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
  });

  it('retains a development copy alongside an optional runtime API', () => {
    const pkg = {
      optionalDependencies: { '@oxlint/plugins': '^1.79.0' },
      devDependencies: { '@oxlint/plugins': '^1.79.0' },
    };
    const packageJsonPath = path.join(projectPath, 'package.json');
    fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));

    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
  });

  it('keeps optional oxlint on the existing tool-migration path', () => {
    const pkg = { optionalDependencies: { oxlint: '^1.79.0' } };
    expect(packageOwnsOxlintApi(pkg)).toBe(false);
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.optionalDependencies).toEqual({});
  });
});

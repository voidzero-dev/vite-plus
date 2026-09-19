import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager, type WorkspaceInfo } from '../../types/index.ts';
import { readJsonFile, writeJsonFile } from '../../utils/json.ts';
import {
  collectOxlintOwnerDirs,
  dropDeadOxlintPluginsDependency,
  finalizeCoreMigrationForExistingVitePlus,
  packageOwnsOxlintApi,
  rewriteMonorepo,
  rewritePackageJson,
  rewriteStandaloneProject,
  sourceTreeReferencesOxlintPluginsPackage,
  usesVitestBrowserMode,
  type DependencyBag,
} from '../migrator.ts';

describe('Oxlint plugin dependency cleanup', () => {
  let projectPath: string;

  beforeEach(() => {
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxlint-plugin-dependency-'));
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
  });

  it.each(['@oxlint/plugins', 'vite-plus/lint/plugins'])(
    'cleans up an existing Vite+ workspace with imports from %s',
    (specifier) => {
      const appPath = path.join(projectPath, 'packages', 'app');
      fs.mkdirSync(appPath, { recursive: true });
      for (const dir of [projectPath, appPath]) {
        writeJsonFile(path.join(dir, 'package.json'), {
          devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' },
        });
        fs.writeFileSync(
          path.join(dir, 'plugin.ts'),
          `import { definePlugin, defineRule, type Context, type ESTree } from '${specifier}';`,
        );
      }
      const workspace = {
        rootDir: projectPath,
        packages: [{ name: 'app', path: 'packages/app' }],
      };

      const result = finalizeCoreMigrationForExistingVitePlus(workspace, true);

      expect(result.imports).toBe(specifier === '@oxlint/plugins');
      expect(result.dependencies).toBe(true);
      for (const dir of [projectPath, appPath]) {
        expect(fs.readFileSync(path.join(dir, 'plugin.ts'), 'utf8')).toContain(
          "from 'vite-plus/lint/plugins'",
        );
        expect(readJsonFile(path.join(dir, 'package.json'))).toEqual({
          devDependencies: { 'vite-plus': 'latest' },
        });
      }
      expect(finalizeCoreMigrationForExistingVitePlus(workspace, true).dependencies).toBe(false);
    },
  );

  it.each([false, true])(
    'cleans up before newly injected browser packages are installed (monorepo: %s)',
    (isMonorepo) => {
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, {
        name: 'project',
        devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' },
      });
      const browserProjectPath = isMonorepo
        ? path.join(projectPath, 'packages', 'app')
        : projectPath;
      if (isMonorepo) {
        fs.mkdirSync(browserProjectPath, { recursive: true });
        fs.writeFileSync(path.join(browserProjectPath, 'package.json'), '{"name":"app"}');
        fs.writeFileSync(
          path.join(projectPath, 'pnpm-workspace.yaml'),
          'packages:\n  - packages/*\n',
        );
      }
      fs.writeFileSync(
        path.join(browserProjectPath, 'browser.ts'),
        "import { playwright } from '@vitest/browser-playwright';",
      );
      fs.writeFileSync(
        path.join(projectPath, 'plugin.ts'),
        "import { defineRule } from '@oxlint/plugins';",
      );
      const workspace: WorkspaceInfo = {
        rootDir: projectPath,
        isMonorepo,
        monorepoScope: '',
        workspacePatterns: isMonorepo ? ['packages/*'] : [],
        parentDirs: [],
        packages: isMonorepo ? [{ name: 'app', path: 'packages/app' }] : [],
        packageManager: PackageManager.pnpm,
        packageManagerVersion: '10.33.0',
        downloadPackageManager: {
          name: PackageManager.pnpm,
          packageName: 'pnpm',
          version: '10.33.0',
          installDir: projectPath,
          binPrefix: projectPath,
        },
      };

      if (isMonorepo) {
        rewriteMonorepo(workspace, true, true);
      } else {
        rewriteStandaloneProject(projectPath, workspace, true, true);
      }

      expect(readJsonFile(packageJsonPath).devDependencies).not.toHaveProperty('@oxlint/plugins');
      expect(
        readJsonFile(path.join(browserProjectPath, 'package.json')).devDependencies,
      ).toHaveProperty('@vitest/browser-playwright');
    },
  );

  it.each([
    { scripts: { 'check-plugin': `node -e "require('@oxlint/plugins')"` } },
    { imports: { '#plugin-api': '@oxlint/plugins' } },
    { dependencies: { '@oxlint/plugins': '^1.79.0' } },
    { peerDependencies: { '@oxlint/plugins': '^1.79.0' } },
    { optionalDependencies: { '@oxlint/plugins': '^1.79.0' } },
  ])('retains an existing Vite+ dependency required by %j', (references) => {
    const pkg = {
      ...references,
      devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' },
    };
    const packageJsonPath = path.join(projectPath, 'package.json');
    writeJsonFile(packageJsonPath, pkg);
    fs.writeFileSync(
      path.join(projectPath, 'plugin.ts'),
      "import { defineRule } from '@oxlint/plugins';",
    );

    const result = finalizeCoreMigrationForExistingVitePlus({ rootDir: projectPath }, true);

    expect(result.dependencies).toBe(false);
    expect(readJsonFile(packageJsonPath)).toEqual(pkg);
    expect(result.imports).toBe(!packageOwnsOxlintApi(pkg));
  });

  it('retains a root dependency used by an ignored nested plugin after finalization', () => {
    const pkg = { devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' } };
    const packageJsonPath = path.join(projectPath, 'package.json');
    writeJsonFile(packageJsonPath, pkg);
    fs.writeFileSync(path.join(projectPath, '.gitignore'), 'dist/\n');
    const outputPath = path.join(projectPath, 'packages', 'app', 'dist');
    fs.mkdirSync(outputPath, { recursive: true });
    fs.writeFileSync(path.join(outputPath, '..', 'package.json'), '{"name":"app"}');
    fs.writeFileSync(
      path.join(outputPath, 'plugin.cjs'),
      "const { defineRule } = require('@oxlint/plugins');",
    );

    const result = finalizeCoreMigrationForExistingVitePlus(
      { rootDir: projectPath, packages: [{ name: 'app', path: 'packages/app' }] },
      true,
    );

    expect(result.dependencies).toBe(false);
    expect(readJsonFile(packageJsonPath)).toEqual(pkg);
  });

  it.each(['dependencies', 'devDependencies', 'optionalDependencies'] as const)(
    'retains the provider of an installed %s plugin required peer',
    (field) => {
      const pkg: DependencyBag = {
        devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' },
      };
      pkg[field] = { ...pkg[field], 'review-oxlint-plugin': '1.0.0' };
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, pkg);
      const pluginPath = path.join(projectPath, 'node_modules', 'review-oxlint-plugin');
      fs.mkdirSync(pluginPath, { recursive: true });
      writeJsonFile(path.join(pluginPath, 'package.json'), {
        name: 'review-oxlint-plugin',
        version: '1.0.0',
        exports: './index.cjs',
        peerDependencies: { '@oxlint/plugins': '^1.79.0' },
      });
      fs.writeFileSync(path.join(pluginPath, 'index.cjs'), "require('@oxlint/plugins');");
      fs.writeFileSync(path.join(projectPath, 'check.cjs'), "require('review-oxlint-plugin');");

      const result = finalizeCoreMigrationForExistingVitePlus({ rootDir: projectPath }, true);

      expect(result.dependencies).toBe(false);
      expect(readJsonFile(packageJsonPath)).toEqual(pkg);
    },
  );

  it.each(['missing', 'invalid', 'optional', 'unrelated'])(
    'handles %s installed peer metadata conservatively',
    (metadata) => {
      const pkg = {
        devDependencies: {
          'vite-plus': 'latest',
          '@oxlint/plugins': '^1.79.0',
          'review-oxlint-plugin': '1.0.0',
        },
      };
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, pkg);
      if (metadata !== 'missing') {
        const pluginPath = path.join(projectPath, 'node_modules', 'review-oxlint-plugin');
        fs.mkdirSync(pluginPath, { recursive: true });
        fs.writeFileSync(
          path.join(pluginPath, 'package.json'),
          metadata === 'invalid'
            ? '{'
            : JSON.stringify({
                name: 'review-oxlint-plugin',
                version: '1.0.0',
                peerDependencies: {
                  [metadata === 'optional' ? '@oxlint/plugins' : 'some-other-api']: '^1.79.0',
                },
                peerDependenciesMeta: { '@oxlint/plugins': { optional: true } },
              }),
        );
      }

      const result = finalizeCoreMigrationForExistingVitePlus({ rootDir: projectPath }, true);
      const keepProvider = metadata === 'missing' || metadata === 'invalid';

      expect(result.dependencies).toBe(!keepProvider);
      expect(
        JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')).devDependencies['@oxlint/plugins'],
      ).toBe(keepProvider ? '^1.79.0' : undefined);
    },
  );

  it.each([false, true])(
    'retains a root peer provider for a nested plugin (workspace: %s)',
    (isWorkspacePackage) => {
      const pkg = { devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' } };
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, pkg);
      const appPath = path.join(projectPath, 'packages', 'app');
      fs.mkdirSync(appPath, { recursive: true });
      writeJsonFile(path.join(appPath, 'package.json'), {
        dependencies: { 'review-oxlint-plugin': '1.0.0' },
      });
      const pluginPath = path.join(appPath, 'node_modules', 'review-oxlint-plugin');
      fs.mkdirSync(pluginPath, { recursive: true });
      writeJsonFile(path.join(pluginPath, 'package.json'), {
        name: 'review-oxlint-plugin',
        peerDependencies: { '@oxlint/plugins': '^1.79.0' },
      });

      const result = finalizeCoreMigrationForExistingVitePlus(
        {
          rootDir: projectPath,
          packages: isWorkspacePackage ? [{ name: 'app', path: 'packages/app' }] : undefined,
        },
        true,
      );

      expect(result.dependencies).toBe(false);
      expect(readJsonFile(packageJsonPath)).toEqual(pkg);
    },
  );

  it.each([{ '.': { import: './index.js' } }, { '.': './dist/index.js' }])(
    'reads installed peer metadata despite inaccessible exports %j',
    (exports) => {
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, {
        devDependencies: {
          'vite-plus': 'latest',
          '@oxlint/plugins': '^1.79.0',
          'review-oxlint-plugin': '1.0.0',
        },
      });
      const pluginPath = path.join(projectPath, 'node_modules', 'review-oxlint-plugin');
      fs.mkdirSync(pluginPath, { recursive: true });
      writeJsonFile(path.join(pluginPath, 'package.json'), {
        name: 'review-oxlint-plugin',
        version: '1.0.0',
        exports,
      });
      fs.writeFileSync(path.join(pluginPath, 'index.js'), 'export default {};');

      const result = finalizeCoreMigrationForExistingVitePlus({ rootDir: projectPath }, true);

      expect(result.dependencies).toBe(true);
      expect(
        JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')).devDependencies['@oxlint/plugins'],
      ).toBeUndefined();
    },
  );

  it.each([false, true])(
    'only retains unknown nested peer contracts for workspace packages (workspace: %s)',
    (isWorkspacePackage) => {
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, {
        devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' },
      });
      const nestedPath = path.join(projectPath, 'nested');
      fs.mkdirSync(nestedPath);
      writeJsonFile(path.join(nestedPath, 'package.json'), {
        name: 'nested',
        devDependencies: { 'uninstalled-plugin': '1.0.0' },
      });

      const result = finalizeCoreMigrationForExistingVitePlus(
        {
          rootDir: projectPath,
          packages: isWorkspacePackage ? [{ name: 'nested', path: 'nested' }] : undefined,
        },
        true,
      );

      expect(result.dependencies).toBe(!isWorkspacePackage);
      expect(
        JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')).devDependencies['@oxlint/plugins'],
      ).toBe(isWorkspacePackage ? '^1.79.0' : undefined);
    },
  );

  it.each(['#!/usr/bin/env node\n', ''])(
    'retains an extensionless Node script with prefix %j',
    (prefix) => {
      const pkg = {
        devDependencies: { 'vite-plus': 'latest', '@oxlint/plugins': '^1.79.0' },
        scripts: { 'check-plugin': 'node bin/check-plugin' },
      };
      const packageJsonPath = path.join(projectPath, 'package.json');
      writeJsonFile(packageJsonPath, pkg);
      fs.mkdirSync(path.join(projectPath, 'bin'));
      const scriptPath = path.join(projectPath, 'bin', 'check-plugin');
      const script = `${prefix}console.log(typeof require('@oxlint/plugins').defineRule);`;
      fs.writeFileSync(scriptPath, script);

      const result = finalizeCoreMigrationForExistingVitePlus({ rootDir: projectPath }, true);

      expect(result.dependencies).toBe(false);
      expect(fs.readFileSync(scriptPath, 'utf8')).toBe(script);
      expect(readJsonFile(packageJsonPath)).toEqual(pkg);
    },
  );

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

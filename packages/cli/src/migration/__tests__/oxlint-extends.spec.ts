import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  injectLintTypeCheckDefaults,
  mergeViteConfigFiles,
  resolveOxlintConfigExtends,
  rewriteMonorepo,
} from '../migrator.ts';
import { createMigrationReport } from '../report.ts';

describe('Oxlint JSON extends migration', () => {
  let project: string;

  function write(filename: string, content: string | object) {
    const filenamePath = path.join(project, filename);
    fs.mkdirSync(path.dirname(filenamePath), { recursive: true });
    fs.writeFileSync(filenamePath, typeof content === 'string' ? content : JSON.stringify(content));
    return filenamePath;
  }

  beforeEach(() => {
    project = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxlint-extends-'));
    write('package.json', { name: 'extends-test', type: 'module' });
    write('vite.config.mjs', 'export default {};');
  });

  afterEach(() => {
    fs.rmSync(project, { recursive: true, force: true });
  });

  it('resolves nested JSONC paths relative to each declaring file and retains inheritance order', async () => {
    const base = write(
      'shared/base.jsonc',
      `{
      // Inherited options must not be replaced by migration defaults.
      "$schema": "./schema.json",
      "options": { "typeAware": false, "typeCheck": false },
      "rules": { "no-debugger": "error" },
    }`,
    );
    write('shared/nested/first.json', {
      extends: ['../base.jsonc'],
      rules: { 'no-console': 'warn' },
      overrides: [{ files: ['**/*.test.js'], rules: { 'no-debugger': 'off' } }],
    });
    write('second.json', { rules: { 'no-console': 'error' } });
    write('.oxlintrc.json', {
      extends: ['./shared/nested/first.json', './second.json'],
      rules: { 'no-alert': 'off' },
    });

    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);
    const configPath = path.join(project, 'vite.config.mjs');
    const { default: config } = await import(pathToFileURL(configPath).href);
    expect(config.lint).toMatchObject({
      extends: [
        {
          extends: [
            { options: { typeAware: false, typeCheck: false }, rules: { 'no-debugger': 'error' } },
          ],
          rules: { 'no-console': 'warn' },
          overrides: [{ files: ['**/*.test.js'], rules: { 'no-debugger': 'off' } }],
        },
        { rules: { 'no-console': 'error' } },
      ],
      options: {},
      rules: { 'no-alert': 'off' },
    });
    expect(JSON.stringify(config)).not.toContain('$schema');
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(false);
    expect(fs.readFileSync(base, 'utf8')).toContain('// Inherited options');
    expect(report.mergedConfigCount).toBe(1);
    expect(report.warnings).toEqual([]);

    const migrated = fs.readFileSync(configPath, 'utf8');
    mergeViteConfigFiles(project, true, report);
    injectLintTypeCheckDefaults(project, true, report);
    expect(fs.readFileSync(configPath, 'utf8')).toBe(migrated);
    expect(report.mergedConfigCount).toBe(1);
  });

  it('allows a shared ancestor to occur in separate branches', () => {
    write('base.json', { rules: { 'no-debugger': 'error' } });
    write('a.json', { extends: ['./base.json'] });
    write('b.json', { extends: ['./base.json'] });
    const root = write('.oxlintrc.json', { extends: ['./a.json', './b.json'] });
    expect(resolveOxlintConfigExtends(root).extends).toEqual([
      { extends: [{ rules: { 'no-debugger': 'error' } }] },
      { extends: [{ rules: { 'no-debugger': 'error' } }] },
    ]);
  });

  it.each([false, true])(
    'resolves workspace inheritance before configs are removed (extends root: %s)',
    async (extendsRoot) => {
      write('package.json', { name: 'root', private: true, workspaces: ['packages/*'] });
      for (const name of ['a', 'b']) {
        write(`packages/${name}/package.json`, { name });
        write(`packages/${name}/vite.config.mjs`, 'export default {};');
      }
      write('packages/a/.oxlintrc.json', { rules: { 'no-debugger': 'error' } });
      write('packages/b/.oxlintrc.json', {
        extends: [extendsRoot ? '../../.oxlintrc.json' : '../a/.oxlintrc.json'],
      });
      write('.oxlintrc.json', {
        extends: [extendsRoot ? './packages/a/.oxlintrc.json' : './packages/b/.oxlintrc.json'],
      });
      const report = createMigrationReport();

      rewriteMonorepo(
        {
          rootDir: project,
          isMonorepo: true,
          monorepoScope: '',
          workspacePatterns: ['packages/*'],
          parentDirs: ['packages'],
          packageManager: PackageManager.npm,
          packageManagerVersion: '11.0.0',
          downloadPackageManager: {
            name: PackageManager.npm,
            installDir: project,
            binPrefix: project,
            packageName: 'npm',
            version: '11.0.0',
          },
          packages: [
            { name: 'a', path: 'packages/a' },
            { name: 'b', path: 'packages/b' },
          ],
        },
        true,
        true,
        report,
      );

      const { default: root } = await import(
        pathToFileURL(path.join(project, 'vite.config.mjs')).href
      );
      const { default: sibling } = await import(
        pathToFileURL(path.join(project, 'packages/b/vite.config.mjs')).href
      );
      const inheritedRoot = extendsRoot ? root.lint.extends[0] : root.lint.extends[0].extends[0];
      const inheritedSibling = extendsRoot
        ? sibling.lint.extends[0].extends[0]
        : sibling.lint.extends[0];
      expect(inheritedRoot.rules['no-debugger']).toBe('error');
      expect(inheritedSibling.rules['no-debugger']).toBe('error');
      for (const dir of ['', 'packages/a', 'packages/b']) {
        expect(fs.existsSync(path.join(project, dir, '.oxlintrc.json'))).toBe(false);
      }
      expect(report.warnings).toEqual([]);
    },
  );

  it('retains a bare plugin when it resolves to the same package from both directories', () => {
    write('node_modules/eslint-plugin-test/package.json', {
      name: 'eslint-plugin-test',
      main: 'index.js',
    });
    write('node_modules/eslint-plugin-test/index.js', 'module.exports = {};');
    write('shared/base.json', {
      jsPlugins: ['eslint-plugin-test'],
      rules: { 'test/custom': 'error' },
    });
    const configPath = write('.oxlintrc.json', { extends: ['./shared/base.json'] });

    expect(resolveOxlintConfigExtends(configPath).extends?.[0]).toEqual({
      jsPlugins: ['eslint-plugin-test'],
      rules: { 'test/custom': 'error' },
    });
  });

  it('preserves configs whose inherited package-local plugin would resolve differently', () => {
    write('shared/node_modules/eslint-plugin-test/package.json', {
      name: 'eslint-plugin-test',
      main: 'index.js',
    });
    write('shared/node_modules/eslint-plugin-test/index.js', 'module.exports = {};');
    write('shared/base.json', { jsPlugins: ['eslint-plugin-test'] });
    const source = write('.oxlintrc.json', { extends: ['./shared/base.json'] });
    const original = fs.readFileSync(source, 'utf8');
    const report = createMigrationReport();

    mergeViteConfigFiles(project, true, report);

    expect(fs.readFileSync(source, 'utf8')).toBe(original);
    expect(report.mergedConfigCount).toBe(0);
    expect(report.warnings[0]).toContain('resolves differently');
  });

  it('does not override an inherited opt-out of the default import rule', async () => {
    write('base.json', { rules: { 'vite-plus/prefer-vite-plus-imports': 'off' } });
    write('.oxlintrc.json', { extends: ['./base.json'] });

    mergeViteConfigFiles(project, true);

    const { default: config } = await import(
      pathToFileURL(path.join(project, 'vite.config.mjs')).href
    );
    expect(config.lint.rules['vite-plus/prefer-vite-plus-imports']).toBe('off');
  });

  it('rebases inherited JS plugins, including overrides, without embedding machine paths', async () => {
    write('shared/base.json', {
      jsPlugins: [{ name: 'import', specifier: './plugin.mjs' }],
      overrides: [{ files: ['**/*.js'], jsPlugins: ['../override.mjs'] }],
    });
    write('.oxlintrc.json', {
      extends: ['./shared/base.json'],
      rules: { 'import/custom-rule': 'error' },
    });

    mergeViteConfigFiles(project, true);

    const configPath = path.join(project, 'vite.config.mjs');
    expect(fs.readFileSync(configPath, 'utf8')).not.toContain(project);
    const { default: config } = await import(pathToFileURL(configPath).href);
    expect(config.lint.jsPlugins).toContainEqual({
      name: 'import',
      specifier: './shared/plugin.mjs',
    });
    expect(config.lint.jsPlugins).toContain('./override.mjs');
    expect(config.lint.extends[0].jsPlugins).toBeUndefined();
    expect(config.lint.extends[0].overrides[0].jsPlugins).toBeUndefined();
    expect(config.lint.rules['import/custom-rule']).toBe('error');
  });

  it.each([
    ['missing file', { extends: ['./missing.json'] }],
    ['cycle', { extends: ['./.oxlintrc.json'] }],
    ['invalid extends', { extends: './base.json' }],
    ['object extends in JSON', { extends: [{}] }],
    ['invalid JSON', '{ "extends": ["./base.json"], broken }'],
    ['non-object JSON', '[]'],
  ])('preserves the source and avoids default lint config on %s', (_name, content) => {
    const source = write('.oxlintrc.json', content);
    const original = fs.readFileSync(source, 'utf8');
    const report = createMigrationReport();

    mergeViteConfigFiles(project, true, report);
    injectLintTypeCheckDefaults(project, true, report);

    expect(fs.readFileSync(source, 'utf8')).toBe(original);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toBe(
      'export default {};',
    );
    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toContain('vp lint -c .oxlintrc.json');
    expect(report.mergedConfigCount).toBe(0);
  });

  it('preserves the source when the Vite config cannot be merged', () => {
    write('base.json', { rules: { 'no-debugger': 'error' } });
    const source = write('.oxlintrc.json', { extends: ['./base.json'] });
    const original = fs.readFileSync(source, 'utf8');
    write('vite.config.mjs', "export { default } from './another-config.mjs';");
    const report = createMigrationReport();

    mergeViteConfigFiles(project, true, report);

    expect(fs.readFileSync(source, 'utf8')).toBe(original);
    expect(report.mergedConfigCount).toBe(0);
    expect(report.warnings[0]).toContain('Failed to merge');
    expect(fs.readdirSync(project).some((file) => file.startsWith('.vite-plus-lint-'))).toBe(false);
  });

  it('preserves an inheritance tree when a lint config already exists', () => {
    write('base.json', { rules: { 'no-debugger': 'error' } });
    const source = write('.oxlintrc.json', { extends: ['./base.json'] });
    const original = fs.readFileSync(source, 'utf8');
    const viteConfig = 'export default { lint: { rules: { "no-console": "warn" } } };';
    write('vite.config.mjs', viteConfig);
    const report = createMigrationReport();

    mergeViteConfigFiles(project, true, report);

    expect(fs.readFileSync(source, 'utf8')).toBe(original);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toBe(viteConfig);
    expect(report.warnings[0]).toContain('already has a lint config');
  });
});

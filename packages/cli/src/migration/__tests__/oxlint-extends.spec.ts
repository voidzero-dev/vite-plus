import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { loadConfigFromFile } from 'vite';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

  async function loadConfig(directory = project) {
    const result = await loadConfigFromFile(
      { command: 'build', mode: 'test' },
      path.join(directory, 'vite.config.mjs'),
      directory,
      'silent',
    );
    return result!.config;
  }

  function createWorkspace(names = ['a', 'b']) {
    write('package.json', {
      name: 'root',
      type: 'module',
      private: true,
      workspaces: ['packages/*'],
    });
    for (const name of names) {
      write(`packages/${name}/package.json`, { name, type: 'module' });
      write(`packages/${name}/vite.config.mjs`, 'export default {};');
    }
  }

  function migrateWorkspace(names = ['a', 'b']) {
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
        packages: names.map((name) => ({ name, path: `packages/${name}` })),
      },
      true,
      true,
      report,
    );
    return report;
  }

  beforeEach(() => {
    project = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxlint-extends-')));
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
    const config = await loadConfig();
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

  it('keeps shared module edits live through nested imports', async () => {
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    write('first.json', { extends: ['./base.json'] });
    write('second.json', { extends: ['./base.json'] });
    write('.oxlintrc.json', { extends: ['./first.json', './second.json'] });
    mergeViteConfigFiles(project, true);

    const modulePath = path.join(project, 'base.ts');
    expect(fs.readFileSync(path.join(project, 'first.ts'), 'utf8')).toContain('./base.js');
    expect(fs.readFileSync(path.join(project, 'second.ts'), 'utf8')).toContain('./base.js');
    fs.writeFileSync(modulePath, fs.readFileSync(modulePath, 'utf8').replace('"warn"', '"error"'));
    const config = await loadConfig();
    for (const branch of config.lint!.extends!) {
      expect(branch.extends![0].rules!['no-debugger']).toBe('error');
    }
  });

  it.each(['base.ts', 'base.js'])('preserves configs when %s already exists', (filename) => {
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    const original = 'export default { rules: { "no-debugger": "off" } };';
    write(filename, original);
    write('.oxlintrc.json', { extends: ['./base.json'] });
    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);
    expect(fs.readFileSync(path.join(project, filename), 'utf8')).toBe(original);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toBe(
      'export default {};',
    );
    expect(report.warnings[0]).toContain('already exists');
  });

  it('does not create partial modules when two JSON sources map to the same TS file', () => {
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    write('base.jsonc', { rules: { 'no-console': 'warn' } });
    write('.oxlintrc.json', { extends: ['./base.json', './base.jsonc'] });
    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);
    expect(report.warnings[0]).toContain('Multiple Oxlint configs');
    expect(fs.existsSync(path.join(project, 'base.ts'))).toBe(false);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
  });

  it('avoids import binding conflicts and preserves existing config content', async () => {
    write(
      'vite.config.mjs',
      'const oxlintConfig1 = "original"; export default { custom: oxlintConfig1 };',
    );
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    write('.oxlintrc.json', { extends: ['./base.json'] });
    mergeViteConfigFiles(project, true);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toContain(
      'import oxlintConfig2',
    );
    expect(await loadConfig()).toHaveProperty('custom', 'original');
  });

  it('allows a shared ancestor to occur in separate branches', () => {
    write('base.json', { rules: { 'no-debugger': 'error' } });
    write('a.json', { extends: ['./base.json'] });
    write('b.json', { extends: ['./base.json'] });
    const root = write('.oxlintrc.json', { extends: ['./a.json', './b.json'] });
    expect(resolveOxlintConfigExtends(root).config.extends).toEqual([
      { extends: [{ rules: { 'no-debugger': 'error' } }] },
      { extends: [{ rules: { 'no-debugger': 'error' } }] },
    ]);
  });

  it('encodes URL characters in relative plugin paths', async () => {
    write('base.json', { jsPlugins: ['./plugin #1%.mjs'] });
    write('.oxlintrc.json', { extends: ['./base.json'] });
    mergeViteConfigFiles(project, true);
    expect((await loadConfig()).lint!.extends![0].jsPlugins).toEqual([
      path.join(project, 'plugin #1%.mjs'),
    ]);
  });

  it('preserves external shared configs without writing outside the project', () => {
    write('external/base.json', { rules: { 'no-debugger': 'warn' } });
    write('app/vite.config.mjs', 'export default {};');
    write('app/.oxlintrc.json', { extends: ['../external/base.json'] });
    const report = createMigrationReport();
    mergeViteConfigFiles(path.join(project, 'app'), true, report);
    expect(report.warnings[0]).toContain('outside the project');
    expect(fs.existsSync(path.join(project, 'external/base.ts'))).toBe(false);
    expect(fs.existsSync(path.join(project, 'app/.oxlintrc.json'))).toBe(true);
  });

  it('removes newly created modules if writing a module fails', () => {
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    write('.oxlintrc.json', { extends: ['./base.json'] });
    const report = createMigrationReport();
    const writeFile = fs.writeFileSync;
    const spy = vi.spyOn(fs, 'writeFileSync').mockImplementation((file, data, options) => {
      if (typeof file === 'number') {
        throw new Error('module write failed');
      }
      return writeFile(file, data, options);
    });
    try {
      mergeViteConfigFiles(project, true, report);
    } finally {
      spy.mockRestore();
    }
    expect(report.warnings[0]).toContain('module write failed');
    expect(fs.existsSync(path.join(project, 'base.ts'))).toBe(false);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toBe(
      'export default {};',
    );
  });

  it.each([false, true])(
    'shares live modules with their owning workspace (extends root: %s)',
    async (extendsRoot) => {
      createWorkspace();
      write('packages/a/.oxlintrc.json', { rules: { 'no-debugger': 'error' } });
      write('packages/b/.oxlintrc.json', {
        extends: [extendsRoot ? '../../.oxlintrc.json' : '../a/.oxlintrc.json'],
      });
      write('.oxlintrc.json', {
        extends: [extendsRoot ? './packages/a/.oxlintrc.json' : './packages/b/.oxlintrc.json'],
      });
      const report = migrateWorkspace();
      const modulePath = path.join(project, 'packages/a/.oxlintrc.ts');
      for (const severity of ['error', 'warn']) {
        if (severity === 'warn') {
          fs.writeFileSync(
            modulePath,
            fs.readFileSync(modulePath, 'utf8').replace('"error"', '"warn"'),
          );
        }
        const root = await loadConfig();
        const sibling = await loadConfig(path.join(project, 'packages/b'));
        const owner = await loadConfig(path.join(project, 'packages/a'));
        expect(root.lint!.extends![0].extends![0].rules!['no-debugger']).toBe(severity);
        expect(sibling.lint!.extends![0].extends![0].rules!['no-debugger']).toBe(severity);
        expect(owner.lint!.extends![0].rules!['no-debugger']).toBe(severity);
        expect(owner.lint!.rules!['no-debugger']).toBeUndefined();
        expect(owner.lint!.options).toEqual({ typeAware: true, typeCheck: true });
        expect(owner.lint!.extends![0].options).toBeUndefined();
      }
      for (const dir of ['', 'packages/a', 'packages/b']) {
        expect(fs.existsSync(path.join(project, dir, '.oxlintrc.json'))).toBe(false);
      }
      expect(report.warnings).toEqual([]);
    },
  );

  it('retains transitive JSON dependencies of a workspace whose module output conflicts', async () => {
    createWorkspace(['a', 'b', 'c']);
    const root = write('.oxlintrc.json', { rules: { 'no-debugger': 'error' } });
    const shared = write('packages/a/.oxlintrc.json', { extends: ['../../.oxlintrc.json'] });
    const consumer = write('packages/b/.oxlintrc.json', {
      extends: ['../a/.oxlintrc.json', './extra.json'],
    });
    write('packages/b/extra.json', {});
    write('packages/b/extra.ts', 'export default {};');
    write('packages/c/.oxlintrc.json', { rules: { 'no-console': 'warn' } });
    const original = [root, shared, consumer].map((file) => fs.readFileSync(file, 'utf8'));

    const report = migrateWorkspace(['a', 'b', 'c']);

    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toContain('already exists');
    expect([root, shared, consumer].map((file) => fs.readFileSync(file, 'utf8'))).toEqual(original);
    const fallback = resolveOxlintConfigExtends(consumer);
    expect(fallback.config.extends![0].extends![0].rules!['no-debugger']).toBe('error');
    expect((await loadConfig()).lint!.extends![0].rules!['no-debugger']).toBe('error');
    expect((await loadConfig(path.join(project, 'packages/a'))).lint).toBeDefined();
    expect((await loadConfig(path.join(project, 'packages/b'))).lint).toBeUndefined();
    expect(fs.existsSync(path.join(project, 'packages/c/.oxlintrc.json'))).toBe(false);
    expect(fs.readFileSync(path.join(project, 'packages/b/extra.ts'), 'utf8')).toBe(
      'export default {};',
    );
  });

  it('retains dependencies of shared JSON files even when all workspace consumers migrate', () => {
    createWorkspace(['a']);
    write('.oxlintrc.json', { rules: { 'no-debugger': 'error' } });
    const shared = write('shared.json', { extends: ['./.oxlintrc.json'] });
    write('packages/a/.oxlintrc.json', { extends: ['../../shared.json'] });

    const report = migrateWorkspace(['a']);

    expect(report.warnings).toEqual([]);
    expect(resolveOxlintConfigExtends(shared).config.extends![0].rules!['no-debugger']).toBe(
      'error',
    );
    expect(fs.existsSync(path.join(project, 'packages/a/.oxlintrc.json'))).toBe(false);
  });

  it('preserves JSON candidates when a failed graph leaves its dependencies unknown', () => {
    createWorkspace(['a']);
    const root = write('.oxlintrc.json', { rules: { 'no-debugger': 'error' } });
    write('packages/a/.oxlintrc.json', { extends: ['./missing.json', '../../.oxlintrc.json'] });

    const report = migrateWorkspace(['a']);

    expect(report.warnings).toHaveLength(1);
    expect(fs.existsSync(root)).toBe(true);
  });

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

    expect(resolveOxlintConfigExtends(configPath).config.extends?.[0]).toEqual({
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

    const config = await loadConfig();
    expect(config.lint!.rules!['vite-plus/prefer-vite-plus-imports']).toBeUndefined();
    expect(config.lint!.extends![0].rules!['vite-plus/prefer-vite-plus-imports']).toBe('off');
  });

  it('keeps inherited JS plugins and overrides in their modules with portable absolute paths', async () => {
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
    const config = await loadConfig();
    expect(config.lint!.extends![0].jsPlugins).toEqual([
      {
        name: 'import',
        specifier: path.join(project, 'shared/plugin.mjs'),
      },
    ]);
    expect(config.lint!.extends![0].overrides![0].jsPlugins).toEqual([
      path.join(project, 'override.mjs'),
    ]);
    expect(config.lint!.jsPlugins).toEqual([
      { name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' },
    ]);
    const module = fs.readFileSync(path.join(project, 'shared/base.ts'), 'utf8');
    expect(module).toContain('import.meta.url');
    expect(module).not.toContain(project);
    expect(config.lint!.rules!['import/custom-rule']).toBe('error');
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
    expect(report.warnings[0]).toContain('ESM default export');
    expect(fs.existsSync(path.join(project, 'base.ts'))).toBe(false);
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

  it('creates no shared modules when the config merger does not support the export shape', () => {
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    write('.oxlintrc.json', { extends: ['./base.json'] });
    write('vite.config.mjs', 'const makeConfig = () => ({}); export default makeConfig();');
    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);
    expect(report.warnings[0]).toContain('Failed to merge');
    expect(fs.existsSync(path.join(project, 'base.ts'))).toBe(false);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
  });
});

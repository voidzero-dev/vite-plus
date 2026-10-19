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
  retryPreservedOxlintConfigs,
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

  it.each(['#', '%'])('loads shared config imports with %s in their paths', async (character) => {
    write(`shared${character}/base${character}.json`, {
      rules: { 'no-debugger': 'error' },
    });
    write(`shared${character}/nested/first${character}.json`, {
      extends: [`../base${character}.json`],
    });
    write('.oxlintrc.json', {
      extends: [`./shared${character}/nested/first${character}.json`],
    });

    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);

    expect(report.warnings).toEqual([]);
    const config = await loadConfig();
    expect(config.lint!.extends![0].extends![0].rules!['no-debugger']).toBe('error');
  });

  it.skipIf(process.platform === 'win32')(
    'preserves JSON when Vite cannot resolve a shared config path containing a question mark',
    () => {
      write('shared?/base.json', { rules: { 'no-debugger': 'error' } });
      write('first.json', { extends: ['./shared?/base.json'] });
      write('.oxlintrc.json', { extends: ['./first.json'] });
      const report = createMigrationReport();

      mergeViteConfigFiles(project, true, report);

      expect(report.warnings[0]).toContain('relative path contains "?"');
      expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
      expect(fs.existsSync(path.join(project, 'first.ts'))).toBe(false);
      expect(fs.existsSync(path.join(project, 'shared?/base.ts'))).toBe(false);
      expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toBe(
        'export default {};',
      );
    },
  );

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

  it('keeps ignore patterns live at each shared owner without applying them to dependents', async () => {
    createWorkspace();
    write(
      'vite.config.mjs',
      'const oxlintConfig1 = "original"; export default { custom: oxlintConfig1 };',
    );
    write('.oxlintrc.json', {
      ignorePatterns: ['generated/**'],
      rules: { 'no-debugger': 'error' },
    });
    write('packages/a/.oxlintrc.json', {
      extends: ['../../.oxlintrc.json'],
      ignorePatterns: ['local/**'],
    });
    write('packages/b/.oxlintrc.json', { extends: ['../a/.oxlintrc.json'] });

    const report = migrateWorkspace();

    expect(report.warnings).toEqual([]);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toContain(
      'oxlintConfig2.ignorePatterns',
    );
    for (const updated of [false, true]) {
      if (updated) {
        const rootModule = path.join(project, '.oxlintrc.ts');
        fs.writeFileSync(
          rootModule,
          fs.readFileSync(rootModule, 'utf8').replace('generated/**', 'built/**'),
        );
        const packageModule = path.join(project, 'packages/a/.oxlintrc.ts');
        fs.writeFileSync(
          packageModule,
          fs.readFileSync(packageModule, 'utf8').replace('local/**', 'vendor/**'),
        );
      }
      const owner = await loadConfig();
      const intermediate = await loadConfig(path.join(project, 'packages/a'));
      const dependent = await loadConfig(path.join(project, 'packages/b'));
      expect(owner.lint!.ignorePatterns).toEqual([updated ? 'built/**' : 'generated/**']);
      expect(intermediate.lint!.ignorePatterns).toEqual([updated ? 'vendor/**' : 'local/**']);
      expect(dependent.lint!.ignorePatterns).toBeUndefined();
    }
  });

  it.each([false, true])(
    'migrates both symlinked workspace configs (shared target extends root: %s)',
    async (extendsRoot) => {
      createWorkspace();
      const root = write('.oxlintrc.json', { rules: { 'no-console': 'off' } });
      const shared = write('shared/lint.json', {
        ...(extendsRoot ? { extends: [root] } : {}),
        rules: { 'no-console': 'error' },
        options: { typeAware: false, typeCheck: false },
      });
      const original = fs.readFileSync(shared, 'utf8');
      for (const name of ['a', 'b']) {
        fs.symlinkSync(
          '../../shared/lint.json',
          path.join(project, `packages/${name}/.oxlintrc.json`),
        );
      }

      const report = migrateWorkspace();

      expect(report.warnings).toEqual([]);
      for (const name of ['a', 'b']) {
        const directory = path.join(project, `packages/${name}`);
        const { lint } = await loadConfig(directory);
        expect(lint!.rules!['no-console']).toBe('error');
        expect(lint!.options).toEqual({ typeAware: false, typeCheck: false });
        expect(fs.existsSync(path.join(directory, '.oxlintrc.json'))).toBe(false);
        if (extendsRoot) {
          expect(lint!.extends![0].rules!['no-console']).toBe('off');
        }
      }
      expect(fs.readFileSync(shared, 'utf8')).toBe(original);
      // Removing the links leaves their target on disk, so its dependencies must remain too.
      expect(fs.existsSync(root)).toBe(extendsRoot);
      if (extendsRoot) {
        expect(resolveOxlintConfigExtends(shared).config.extends![0].rules!['no-console']).toBe(
          'off',
        );
      }
    },
  );

  it('preserves a failed symlinked consumer after its sibling completes migration', async () => {
    createWorkspace();
    const root = write('.oxlintrc.json', { rules: { 'no-console': 'off' } });
    write('shared/lint.json', {
      extends: [root],
      rules: { 'no-console': 'error' },
    });
    for (const name of ['a', 'b']) {
      fs.symlinkSync(
        '../../shared/lint.json',
        path.join(project, `packages/${name}/.oxlintrc.json`),
      );
    }
    const original = "export { default } from './another-config.mjs';";
    const viteConfig = write('packages/b/vite.config.mjs', original);

    const report = migrateWorkspace();

    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toContain('ESM default export');
    expect((await loadConfig(path.join(project, 'packages/a'))).lint!.rules!['no-console']).toBe(
      'error',
    );
    expect(fs.existsSync(path.join(project, 'packages/a/.oxlintrc.json'))).toBe(false);
    expect(fs.readFileSync(viteConfig, 'utf8')).toBe(original);
    const preserved = path.join(project, 'packages/b/.oxlintrc.json');
    expect(fs.lstatSync(preserved).isSymbolicLink()).toBe(true);
    expect(resolveOxlintConfigExtends(preserved).config.extends![0].rules!['no-console']).toBe(
      'off',
    );

    write('packages/b/vite.config.mjs', 'export default {};');
    const retryReport = createMigrationReport();
    expect(
      retryPreservedOxlintConfigs(
        project,
        true,
        retryReport,
        ['a', 'b'].map((name) => ({ name, path: `packages/${name}` })),
      ),
    ).toBe(true);
    expect(retryReport.warnings).toEqual([]);
    expect(fs.existsSync(preserved)).toBe(false);
    expect(fs.existsSync(path.join(project, 'shared/lint.json'))).toBe(true);
  });

  it.each(['extends', 'symlink'])(
    'retains an alias referenced through a preserved %s',
    (reference) => {
      createWorkspace();
      write('shared/lint.json', { rules: { 'no-console': 'error' } });
      const alias = path.join(project, 'packages/a/.oxlintrc.json');
      const consumer = path.join(project, 'packages/b/.oxlintrc.json');
      fs.symlinkSync('../../shared/lint.json', alias);
      if (reference === 'symlink') {
        fs.symlinkSync('../a/.oxlintrc.json', consumer);
      } else {
        write('packages/b/.oxlintrc.json', { extends: ['../a/.oxlintrc.json'] });
      }
      write('packages/b/vite.config.mjs', "export { default } from './another-config.mjs';");

      expect(migrateWorkspace().warnings).toHaveLength(1);

      expect(fs.lstatSync(alias).isSymbolicLink()).toBe(true);
      const fallback = resolveOxlintConfigExtends(consumer).config;
      const lint = reference === 'symlink' ? fallback : fallback.extends![0];
      expect(lint.rules!['no-console']).toBe('error');
    },
  );

  it('keeps relative plugins in symlinked roots anchored to their workspace', async () => {
    createWorkspace(['a']);
    write('shared/lint.json', { jsPlugins: ['./plugin.mjs'] });
    fs.symlinkSync('../../shared/lint.json', path.join(project, 'packages/a/.oxlintrc.json'));

    expect(migrateWorkspace(['a']).warnings).toEqual([]);

    const { lint } = await loadConfig(path.join(project, 'packages/a'));
    expect(lint!.jsPlugins).toContain('./plugin.mjs');
  });

  it('resolves each symlinked root inheritance graph from its own directory', async () => {
    createWorkspace();
    write('shared/lint.json', { extends: ['./base.json'] });
    write('shared/base.json', { rules: { 'no-console': 'off' } });
    for (const [name, severity] of [
      ['a', 'error'],
      ['b', 'warn'],
    ]) {
      write(`packages/${name}/base.json`, { rules: { 'no-console': severity } });
      fs.symlinkSync(
        '../../shared/lint.json',
        path.join(project, `packages/${name}/.oxlintrc.json`),
      );
    }

    expect(migrateWorkspace().warnings).toEqual([]);

    expect(
      (await loadConfig(path.join(project, 'packages/a'))).lint!.extends![0].rules!['no-console'],
    ).toBe('error');
    expect(
      (await loadConfig(path.join(project, 'packages/b'))).lint!.extends![0].rules!['no-console'],
    ).toBe('warn');
  });

  it('resolves nested symlink dependencies and plugins from their declared directory', async () => {
    write('shared/lint.json', { extends: ['./base.json'], jsPlugins: ['./plugin.mjs'] });
    write('shared/base.json', { rules: { 'no-console': 'off' } });
    write('base.json', { rules: { 'no-console': 'error' } });
    fs.symlinkSync('./shared/lint.json', path.join(project, 'alias.json'));
    write('.oxlintrc.json', { extends: ['./alias.json'] });

    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);

    expect(report.warnings).toEqual([]);
    const { lint } = await loadConfig();
    expect(lint!.extends![0].extends![0].rules!['no-console']).toBe('error');
    expect(lint!.extends![0].jsPlugins).toEqual([path.join(project, 'plugin.mjs')]);
  });

  it('does not reuse a shared module resolved from a different symlink context', async () => {
    createWorkspace();
    write('shared/lint.json', { extends: ['./base.json'] });
    write('shared/base.json', { rules: { 'no-console': 'off' } });
    for (const [name, severity] of [
      ['a', 'error'],
      ['b', 'warn'],
    ]) {
      write(`packages/${name}/base.json`, { rules: { 'no-console': severity } });
      fs.symlinkSync('../../shared/lint.json', path.join(project, `packages/${name}/alias.json`));
      write(`packages/${name}/.oxlintrc.json`, { extends: ['./alias.json'] });
    }

    const report = migrateWorkspace();

    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toContain('already exists');
    expect(
      (await loadConfig(path.join(project, 'packages/a'))).lint!.extends![0].extends![0].rules![
        'no-console'
      ],
    ).toBe('error');
    expect(
      resolveOxlintConfigExtends(path.join(project, 'packages/b/.oxlintrc.json')).config.extends![0]
        .extends![0].rules!['no-console'],
    ).toBe('warn');
  });

  it('shares module edits with an owner reached through a symlink alias', async () => {
    createWorkspace(['a']);
    write('.oxlintrc.json', { rules: { 'no-console': 'warn' } });
    fs.symlinkSync('../../.oxlintrc.json', path.join(project, 'packages/a/alias.json'));
    write('packages/a/.oxlintrc.json', { extends: ['./alias.json'] });

    expect(migrateWorkspace(['a']).warnings).toEqual([]);

    const modulePath = path.join(project, '.oxlintrc.ts');
    fs.writeFileSync(modulePath, fs.readFileSync(modulePath, 'utf8').replace('"warn"', '"error"'));
    expect((await loadConfig()).lint!.extends![0].rules!['no-console']).toBe('error');
    expect(
      (await loadConfig(path.join(project, 'packages/a'))).lint!.extends![0].rules!['no-console'],
    ).toBe('error');
  });

  it.skipIf(process.platform === 'win32')(
    'preserves literal backslashes in plugin filenames',
    async () => {
      write('base.json', { jsPlugins: ['./plugin\\name.mjs'] });
      write('.oxlintrc.json', { extends: ['./base.json'] });

      mergeViteConfigFiles(project, true);

      expect((await loadConfig()).lint!.extends![0].jsPlugins).toEqual([
        path.join(project, 'plugin\\name.mjs'),
      ]);
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

  it('reuses edited modules on a retry after another workspace could not be merged', async () => {
    createWorkspace();
    write('.oxlintrc.json', { rules: { 'no-debugger': 'warn' } });
    write('packages/a/.oxlintrc.json', { extends: ['../../.oxlintrc.json'] });
    write('packages/b/.oxlintrc.json', { extends: ['../a/.oxlintrc.json'] });
    write('packages/b/vite.config.mjs', "export { default } from './other.mjs';");
    expect(migrateWorkspace().warnings).toHaveLength(1);

    const modulePath = path.join(project, '.oxlintrc.ts');
    const edited = fs.readFileSync(modulePath, 'utf8').replace('"warn"', '"error"');
    fs.writeFileSync(modulePath, edited);
    write('packages/b/vite.config.mjs', 'export default {};');

    const report = createMigrationReport();
    expect(
      retryPreservedOxlintConfigs(
        project,
        true,
        report,
        ['a', 'b'].map((name) => ({ name, path: `packages/${name}` })),
      ),
    ).toBe(true);

    expect(report.warnings).toEqual([]);
    expect(fs.existsSync(path.join(project, 'packages/b/.oxlintrc.json'))).toBe(false);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(false);
    expect(fs.existsSync(path.join(project, 'packages/a/.oxlintrc.json'))).toBe(false);
    expect(fs.readFileSync(modulePath, 'utf8')).toBe(edited);
    const { lint } = await loadConfig(path.join(project, 'packages/b'));
    expect(lint!.extends![0].extends![0].rules!['no-debugger']).toBe('error');
  });

  it.each(['lint block', 'JSON config'])(
    'retains an owner whose %s changed after migration',
    (changed) => {
      createWorkspace(['a']);
      write('.oxlintrc.json', { rules: { 'no-debugger': 'warn' } });
      write('packages/a/.oxlintrc.json', { extends: ['../../.oxlintrc.json'] });
      write('packages/a/vite.config.mjs', "export { default } from './another.mjs';");
      expect(migrateWorkspace(['a']).warnings).toHaveLength(1);
      fs.unlinkSync(path.join(project, 'packages/a/.oxlintrc.json'));
      if (changed === 'lint block') {
        write('vite.config.mjs', 'export default { lint: { rules: { "no-debugger": "off" } } };');
      } else {
        write('.oxlintrc.json', { rules: { 'no-debugger': 'error' } });
      }

      expect(retryPreservedOxlintConfigs(project, true)).toBe(false);

      expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
    },
  );

  it('does not retry unrelated JSON configs in an existing Vite+ project', () => {
    write('base.json', { rules: { 'no-debugger': 'error' } });
    write('.oxlintrc.json', { extends: ['./base.json'] });
    const report = createMigrationReport();

    expect(retryPreservedOxlintConfigs(project, true, report)).toBe(false);
    expect(report.warnings).toEqual([]);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
    expect(fs.existsSync(path.join(project, 'base.ts'))).toBe(false);
    expect(fs.readFileSync(path.join(project, 'vite.config.mjs'), 'utf8')).toBe(
      'export default {};',
    );
  });

  it('does not reuse a generated module for a different JSON source', () => {
    write('base.json', { rules: { 'no-debugger': 'warn' } });
    write('.oxlintrc.json', { extends: ['./base.json'] });
    mergeViteConfigFiles(project, true);
    const original = fs.readFileSync(path.join(project, 'base.ts'), 'utf8');
    write('base.jsonc', { rules: { 'no-debugger': 'error' } });
    write('vite.config.mjs', 'export default {};');
    write('.oxlintrc.json', { extends: ['./base.jsonc'] });

    const report = createMigrationReport();
    mergeViteConfigFiles(project, true, report);

    expect(report.warnings[0]).toContain('already exists');
    expect(fs.readFileSync(path.join(project, 'base.ts'), 'utf8')).toBe(original);
    expect(fs.existsSync(path.join(project, '.oxlintrc.json'))).toBe(true);
  });

  it('keeps owner env, globals, and settings live without applying them to dependents', async () => {
    createWorkspace(['a']);
    write('.oxlintrc.json', {
      env: { browser: true },
      globals: { fixtureGlobal: 'readonly' },
      settings: { fixture: { enabled: true } },
    });
    write('packages/a/.oxlintrc.json', { extends: ['../../.oxlintrc.json'] });
    expect(migrateWorkspace(['a']).warnings).toEqual([]);

    for (const enabled of [true, false]) {
      if (!enabled) {
        const modulePath = path.join(project, '.oxlintrc.ts');
        fs.writeFileSync(
          modulePath,
          fs
            .readFileSync(modulePath, 'utf8')
            .replace('"browser": true', '"browser": false')
            .replace('"readonly"', '"off"')
            .replace('"enabled": true', '"enabled": false'),
        );
      }
      const owner = await loadConfig();
      const dependent = await loadConfig(path.join(project, 'packages/a'));
      expect(owner.lint!.env).toEqual({ browser: enabled });
      expect(owner.lint!.globals).toEqual({ fixtureGlobal: enabled ? 'readonly' : 'off' });
      expect(owner.lint!.settings).toEqual({ fixture: { enabled } });
      expect(dependent.lint!.env).toBeUndefined();
      expect(dependent.lint!.globals).toBeUndefined();
      expect(dependent.lint!.settings).toBeUndefined();
    }
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

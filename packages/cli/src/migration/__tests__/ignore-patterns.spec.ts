import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { VITEST_VERSION, VITE_PLUS_OVERRIDE_PACKAGES } from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { readYamlFile } from '../../utils/yaml.ts';
import { editJsonFile, editYamlFile } from '../files.ts';
import { formatMigratedProject } from '../format.ts';
import { mergeTsdownConfigFile, rewriteAllImports } from '../migrator.ts';
import { createMigrationReport } from '../report.ts';
import { checkEnclosingMigrationPolicy, loadMigrationScope, withMigrationScope } from '../scope.ts';

describe('migration exclusions', () => {
  let root: string;
  const write = (file: string, content: string) => {
    const filename = path.join(root, file);
    fs.mkdirSync(path.dirname(filename), { recursive: true });
    fs.writeFileSync(filename, content);
    return filename;
  };
  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migration-ignore-'));
    write('package.json', '{"private":true}');
    write(
      'vite.config.ts',
      `export default { migrate: { ignorePatterns: ['templates/', 'keep.config.ts'] } };`,
    );
    write('templates/starter/package.json', '{"name":"starter"}');
  });
  afterEach(() => fs.rmSync(root, { recursive: true, force: true }));

  it('loads a static policy without executing uninstalled plugins', async () => {
    write(
      'vite.config.ts',
      `import { defineConfig } from 'vite';
import missing from 'uninstalled-plugin';
export default defineConfig({ plugins: [missing()], migrate: { ignorePatterns: ['templates/'] } });`,
    );
    const scope = await loadMigrationScope(root);
    expect(scope.ignores(path.join(root, 'templates/starter/vite.config.ts'))).toBe(true);
  });

  it('rejects explicit excluded targets while allowing an independent clone', async () => {
    const target = path.join(root, 'templates/starter');
    await expect(checkEnclosingMigrationPolicy(target)).rejects.toThrow(
      'excluded by migrate.ignorePatterns',
    );
    fs.mkdirSync(path.join(target, '.git'));
    await expect(checkEnclosingMigrationPolicy(target)).resolves.toBeUndefined();
  });

  it('preserves excluded source and rewrites included source, including a scan below the root', async () => {
    const excluded = write('templates/starter/test.ts', `import { test } from 'vitest';`);
    const included = write('app/test.ts', `import { test } from 'vitest';`);
    withMigrationScope(await loadMigrationScope(root), () => {
      expect(rewriteAllImports(root, true)).toBe(true);
      expect(rewriteAllImports(path.dirname(excluded), true)).toBe(false);
    });
    expect(fs.readFileSync(excluded, 'utf8')).toBe(`import { test } from 'vitest';`);
    expect(fs.readFileSync(included, 'utf8')).toBe(`import { test } from 'vite-plus/test';`);
  });

  it('preserves shared catalogs and overrides in YAML and JSON, including nested Bun catalogs', async () => {
    const workspace = write(
      'pnpm-workspace.yaml',
      'catalog:\n  vite: ^8.0.0\ncatalogs:\n  old:\n    vitest: ^4.0.0\noverrides:\n  unrelated: 1.0.0\n',
    );
    const manifest = write(
      'package.json',
      JSON.stringify({
        private: true,
        workspaces: { packages: ['templates/*'], catalogs: { old: { vitest: '^4.0.0' } } },
        pnpm: { overrides: { unrelated: '1.0.0' } },
      }),
    );
    const before = readYamlFile(workspace);
    withMigrationScope(await loadMigrationScope(root), () => {
      editYamlFile(workspace, (doc) => {
        doc.setIn(['catalog', 'vite'], VITE_PLUS_OVERRIDE_PACKAGES.vite);
        doc.delete('catalogs');
        doc.setIn(['overrides', 'vite'], VITE_PLUS_OVERRIDE_PACKAGES.vite);
        doc.set('unrelatedSetting', true);
      });
      editJsonFile(manifest, (pkg) => {
        pkg.workspaces = { packages: ['templates/*'] };
        delete pkg.pnpm;
        pkg.overrides = { vite: VITE_PLUS_OVERRIDE_PACKAGES.vite };
        pkg.devDependencies = { 'vite-plus': 'latest' };
        return pkg;
      });
    });
    expect(readYamlFile(workspace)).toEqual({ ...before, unrelatedSetting: true });
    expect(readJsonFile(manifest)).toMatchObject({
      workspaces: { catalogs: { old: { vitest: '^4.0.0' } } },
      pnpm: { overrides: { unrelated: '1.0.0' } },
      devDependencies: { 'vite-plus': 'latest' },
    });
    expect(readJsonFile(manifest)).not.toHaveProperty('overrides');
  });

  it.each([
    { overrides: { 'vite@*': '^8.0.0' } },
    { resolutions: { '**/vitest': '^4.0.0' } },
    { overrides: { 'vite-plus': { vitest: '^4.0.0' } } },
    { overrides: { vitest: { '.': '^4.0.0' } } },
  ])('rejects conflicting shared overrides before any writes: %j', async (settings) => {
    const file = write('package.json', JSON.stringify(settings));
    const before = fs.readFileSync(file, 'utf8');
    await expect(loadMigrationScope(root)).rejects.toThrow('No project files were changed.');
    expect(fs.readFileSync(file, 'utf8')).toBe(before);
  });

  it('accepts compatible global overrides and preserves overrides scoped to an excluded package', async () => {
    write(
      'package.json',
      JSON.stringify({
        overrides: {
          vitest: VITEST_VERSION,
          starter: { vitest: '^4.0.0' },
        },
      }),
    );
    await expect(loadMigrationScope(root)).resolves.toBeDefined();
  });

  it('does not consume a source config when its destination is ignored', async () => {
    write('vite.config.ts', `export default { migrate: { ignorePatterns: ['vite.config.ts'] } };`);
    const source = write('tsdown.config.json', '{"entry":["src/index.ts"]}');
    withMigrationScope(await loadMigrationScope(root), () => {
      expect(mergeTsdownConfigFile(root, true)).toBe(false);
    });
    expect(fs.readFileSync(source, 'utf8')).toBe('{"entry":["src/index.ts"]}');
  });

  it('filters automatic formatting outside Git and when explicit changed paths are supplied', async () => {
    write('app.ts', 'export const app=1;');
    write('templates/starter/app.ts', 'export const template=1;');
    const scope = await loadMigrationScope(root);
    for (const changed of [undefined, ['app.ts', 'templates/starter/app.ts']]) {
      const formatted: string[] = [];
      await withMigrationScope(scope, () =>
        formatMigratedProject(root, false, createMigrationReport(), {
          collectPaths: async () => changed,
          format: async (_cwd, _interactive, paths) => {
            expect(paths).toBeDefined();
            formatted.push(...paths!);
            return { status: 'formatted', durationMs: 0 };
          },
        }),
      );
      expect(formatted).toContain('app.ts');
      expect(
        formatted.some(
          (file) => file.startsWith(`templates${path.sep}`) || file.startsWith('templates/'),
        ),
      ).toBe(false);
    }
  });
});

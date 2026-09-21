import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import semver from 'semver';
import { afterEach, describe, expect, it } from 'vitest';
import { parse as yaml } from 'yaml';

import { PackageManager } from '../../types/index.ts';
import {
  applyVitestV5Migration,
  finishVitestV5Migration,
  planVitestV5Migration,
  vitestV5NeedsMigration,
} from '../migrator.ts';
import { webdriverioMigrationSpec } from '../vitest-v5/webdriverio.ts';

const provider = '@vitest/browser-webdriverio';
const directories: string[] = [];
afterEach(() => {
  for (const dir of directories.splice(0)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
function project(pkg: Record<string, unknown>, files: Record<string, string> = {}) {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-webdriverio-migrate-'));
  directories.push(rootDir);
  for (const [file, text] of Object.entries({
    'package.json': JSON.stringify({ name: 'app', ...pkg }),
    'node_modules/vitest/package.json': JSON.stringify({ name: 'vitest', version: '4.1.11' }),
    ...files,
  })) {
    fs.mkdirSync(path.dirname(path.join(rootDir, file)), { recursive: true });
    fs.writeFileSync(path.join(rootDir, file), text);
  }
  return rootDir;
}
function read(root: string, file = 'package.json') {
  const text = fs.readFileSync(path.join(root, file), 'utf8');
  return file.endsWith('.json') ? JSON.parse(text) : yaml(text);
}
function migrate(
  rootDir: string,
  packageManager: PackageManager = PackageManager.pnpm,
  packages?: { name: string; path: string }[],
) {
  const workspace = { rootDir, packageManager, packages };
  const plan = planVitestV5Migration(workspace);
  expect(plan.findings).toEqual([]);
  applyVitestV5Migration(plan);
  expect(finishVitestV5Migration(plan)).toEqual([]);
  expect(planVitestV5Migration(workspace).changes).toEqual([]);
  return plan;
}

describe('WebDriverIO migration floor', () => {
  it.each(['4.1.11', '^4.1.11', '~4.0.0', '5.0.0-beta.5'])('upgrades %s', (spec) => {
    expect(webdriverioMigrationSpec(spec)).toBe('^5.0.0');
  });
  it.each(['5.0.0', '^5.2.0', '~5.1.1', '^6.0.0', '>=5', '5 || 6'])('preserves %s', (spec) => {
    expect(webdriverioMigrationSpec(spec)).toBe(spec);
  });
  it.each(['*', '>=4', '^4 || ^5.2', '4.1.11 || 6.1.0', '^5.0.0-beta.5'])(
    'narrows %s without allowing v4',
    (spec) => {
      const result = webdriverioMigrationSpec(spec)!;
      expect(semver.subset(result, '>=5.0.0')).toBe(true);
      for (const version of ['5.0.0', '5.2.0', '6.1.0', '7.0.0']) {
        expect(semver.satisfies(version, result)).toBe(semver.satisfies(version, spec));
      }
      expect(webdriverioMigrationSpec(result)).toBe(result);
    },
  );
  it('preserves registry alias identity', () => {
    expect(webdriverioMigrationSpec(`npm:${provider}@^4.1.11`)).toBe(`npm:${provider}@^5.0.0`);
  });

  it.each([PackageManager.npm, PackageManager.pnpm, PackageManager.yarn, PackageManager.bun])(
    'upgrades v4 on %s and composes manifest runtime edits',
    (manager) => {
      const root = project({
        devDependencies: { vitest: '4.1.11', [provider]: '^4.1.11', webdriverio: '^9.0.0' },
        devEngines: { runtime: { name: 'node', version: '20.19.0' } },
        scripts: { bench: 'vitest bench --run' },
      });
      migrate(root, manager);
      const pkg = read(root);
      expect(pkg.devDependencies[provider]).toBe('^5.0.0');
      expect(pkg.devDependencies.webdriverio).toBe('^9.0.0');
      expect(pkg.devEngines.runtime.version).not.toBe('20.19.0');
      expect(pkg.scripts.bench).toBe('vitest bench --run');
    },
  );
  it.each(['dependencies', 'devDependencies', 'optionalDependencies'])(
    'updates %s in place',
    (field) => {
      const root = project({ [field]: { [provider]: '4.1.11', webdriverio: '^9.0.0' } });
      migrate(root);
      expect(read(root)[field][provider]).toBe('^5.0.0');
    },
  );
  it('migrates a Vite+ 0.x source-only import and reuses a WebDriverIO sibling version', () => {
    const root = project(
      { devDependencies: { 'vite-plus': '0.3.2', '@wdio/cli': '^9.20.0' } },
      {
        'vite.config.ts':
          "import { webdriverio } from 'vite-plus/test/browser-webdriverio'; export default {};",
      },
    );
    const plan = migrate(root);
    expect(vitestV5NeedsMigration(plan)).toBe(true);
    expect(read(root).devDependencies).toMatchObject({
      [provider]: '^5.0.0',
      webdriverio: '^9.20.0',
    });
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toContain(
      `from "${provider}"`,
    );
  });
  it('preserves newer provider and framework declarations', () => {
    const root = project({
      devDependencies: { vitest: '5.0.1', [provider]: '^6.0.0', webdriverio: '^10.0.0' },
    });
    const plan = migrate(root);
    expect(plan.changes).toEqual([]);
    expect(vitestV5NeedsMigration(plan)).toBe(false);
  });
  it('adds a dev dependency without narrowing a public peer range', () => {
    const root = project({
      peerDependencies: { [provider]: '^4 || ^5' },
      devDependencies: { webdriverio: '^9' },
    });
    migrate(root);
    expect(read(root).peerDependencies[provider]).toBe('^4 || ^5');
    expect(semver.subset(read(root).devDependencies[provider], '>=5.0.0')).toBe(true);
  });
  it('reuses a newer peer-only provider without downgrading it', () => {
    const root = project({
      peerDependencies: { [provider]: '^6.2.0' },
      devDependencies: { webdriverio: '^9' },
    });
    migrate(root);
    expect(read(root).devDependencies[provider]).toBe('^6.2.0');
  });
  it('reuses a root provider catalog for a source-only workspace member', () => {
    const root = project(
      { devDependencies: { [provider]: 'catalog:browser', webdriverio: '^9' } },
      {
        'pnpm-workspace.yaml': JSON.stringify({ catalogs: { browser: { [provider]: '^6.2.0' } } }),
        'packages/app/package.json': JSON.stringify({
          name: 'child',
          devDependencies: { vitest: '4.1.11', webdriverio: '^9' },
        }),
        'packages/app/vitest.config.ts': `import { webdriverio } from '${provider}'; export default {};`,
      },
    );
    migrate(root, PackageManager.pnpm, [{ name: 'child', path: 'packages/app' }]);
    expect(read(root, 'packages/app/package.json').devDependencies[provider]).toBe(
      'catalog:browser',
    );
    expect(read(root, 'pnpm-workspace.yaml').catalogs.browser[provider]).toBe('^6.2.0');
  });
  it('keeps a compatible npm override unchanged', () => {
    const root = project({
      devDependencies: { [provider]: '5.2.0', webdriverio: '^9' },
      overrides: { [provider]: '5.2.0' },
    });
    const plan = migrate(root, PackageManager.npm);
    expect(read(root).overrides[provider]).toBe('5.2.0');
    expect(plan.findings).toEqual([]);
  });
  it('updates npm long-form pins without changing unrelated children', () => {
    const root = project({
      devDependencies: { [provider]: '4.1.11', webdriverio: '^9' },
      overrides: { [provider]: { '.': '4.1.11', other: '1.0.0' } },
    });
    migrate(root, PackageManager.npm);
    expect(read(root).overrides[provider]).toEqual({ other: '1.0.0' });
  });
  it.each([PackageManager.pnpm, PackageManager.yarn, PackageManager.bun])(
    'updates referenced catalogs on %s',
    (manager) => {
      const catalogs = {
        catalog: { [provider]: '4.1.11' },
        catalogs: { browser: { [provider]: '^4.1.11' }, unused: { [provider]: '4.0.0' } },
      };
      const configFile = manager === PackageManager.yarn ? '.yarnrc.yml' : 'pnpm-workspace.yaml';
      const root = project(
        {
          devDependencies: { [provider]: 'catalog:', webdriverio: '^9' },
          ...(manager === PackageManager.bun
            ? { workspaces: { packages: ['packages/*'], ...catalogs } }
            : {}),
        },
        {
          ...(manager === PackageManager.bun ? {} : { [configFile]: JSON.stringify(catalogs) }),
          'packages/browser/package.json': JSON.stringify({
            name: 'browser',
            devDependencies: { [provider]: 'catalog:browser', webdriverio: '^9' },
          }),
        },
      );
      migrate(root, manager, [{ name: 'browser', path: 'packages/browser' }]);
      const output =
        manager === PackageManager.bun ? read(root).workspaces : read(root, configFile);
      expect(output.catalog[provider]).toBe('^5.0.0');
      expect(output.catalogs.browser[provider]).toBe('^5.0.0');
      expect(output.catalogs.unused[provider]).toBe('4.0.0');
      expect(read(root).devDependencies[provider]).toBe('catalog:');
      expect(read(root, 'packages/browser/package.json').devDependencies[provider]).toBe(
        'catalog:browser',
      );
    },
  );
  it.each([PackageManager.npm, PackageManager.pnpm, PackageManager.yarn, PackageManager.bun])(
    'updates applicable override pins on %s only',
    (manager) => {
      const overrides = {
        [provider]: '4.1.11',
        [`app>${provider}`]: '^4',
        [`unrelated>${provider}`]: '4.0.0',
        unrelated: { [provider]: '4.0.0' },
      };
      const field = manager === PackageManager.yarn ? 'resolutions' : 'overrides';
      const root = project({
        devDependencies: { [provider]: '^5.2.0', webdriverio: '^9' },
        [field]: overrides,
      });
      migrate(root, manager);
      const output = read(root)[field];
      expect(output[provider]).toBeUndefined();
      expect(output[`app>${provider}`]).toBeUndefined();
      expect(output[`unrelated>${provider}`]).toBe('4.0.0');
      expect(output.unrelated[provider]).toBe('4.0.0');
      expect(read(root).devDependencies[provider]).toBe('^5.2.0');
    },
  );
  it.each([PackageManager.npm, PackageManager.pnpm])(
    'removes an old alias override without downgrading a newer provider on %s',
    (manager) => {
      const root = project({
        devDependencies: { [provider]: '^6', webdriverio: '^9' },
        overrides: { [provider]: `npm:${provider}@4.1.11` },
      });
      migrate(root, manager);
      expect(read(root).overrides[provider]).toBeUndefined();
      expect(read(root).devDependencies[provider]).toBe('^6');
    },
  );
  it('removes scoped pnpm catalog overrides without downgrading a newer provider', () => {
    const root = project(
      { devDependencies: { [provider]: '^6', webdriverio: '^9' } },
      {
        'pnpm-workspace.yaml': JSON.stringify({
          catalogs: { old: { [provider]: '4.1.11' } },
          overrides: {
            [`app>${provider}`]: 'catalog:old',
            [`unrelated>${provider}`]: '4.0.0',
          },
        }),
      },
    );
    migrate(root);
    expect(read(root, 'pnpm-workspace.yaml').catalogs.old[provider]).toBe('4.1.11');
    expect(read(root, 'pnpm-workspace.yaml').overrides[`app>${provider}`]).toBeUndefined();
    expect(read(root, 'pnpm-workspace.yaml').overrides[`unrelated>${provider}`]).toBe('4.0.0');
    expect(read(root).devDependencies[provider]).toBe('^6');
  });
  it('retains an override that shares the upgraded dependency catalog', () => {
    const root = project(
      { devDependencies: { [provider]: 'catalog:browser', webdriverio: '^9' } },
      {
        'pnpm-workspace.yaml': JSON.stringify({
          catalogs: { browser: { [provider]: '4.1.11' } },
          overrides: { [provider]: 'catalog:browser' },
        }),
      },
    );
    migrate(root);
    expect(read(root, 'pnpm-workspace.yaml').catalogs.browser[provider]).toBe('^5.0.0');
    expect(read(root, 'pnpm-workspace.yaml').overrides[provider]).toBe('catalog:browser');
  });
  it.each(['latest', 'git+https://example.com/provider.git', 'catalog:missing', 'file:./missing'])(
    'blocks unverified %s before any writes',
    (spec) => {
      const root = project({ devDependencies: { vitest: '4.1.11', [provider]: spec } });
      const before = fs.readFileSync(path.join(root, 'package.json'), 'utf8');
      const plan = planVitestV5Migration({ rootDir: root, packageManager: PackageManager.pnpm });
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'browser-provider', severity: 'block' }),
      );
      expect(() => applyVitestV5Migration(plan)).toThrow('blocking');
      expect(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).toBe(before);
    },
  );
  it('checks local provider metadata and guards it against edits after preflight', () => {
    const root = project(
      { devDependencies: { [provider]: 'file:./provider', webdriverio: '^9' } },
      {
        'provider/package.json': JSON.stringify({ name: provider, version: '5.2.0' }),
      },
    );
    const plan = planVitestV5Migration({ rootDir: root });
    expect(plan.findings).toEqual([]);
    fs.writeFileSync(
      path.join(root, 'provider/package.json'),
      JSON.stringify({ name: provider, version: '4.1.11' }),
    );
    expect(() => applyVitestV5Migration(plan)).toThrow('Migration input changed');
  });
  it('does not infer browser usage from generated Yarn PnP files', () => {
    const root = project(
      { devDependencies: { vitest: '5.0.1' } },
      {
        '.pnp.cjs': `module.exports = { optionalPeer: '${provider}' };`,
        '.pnp.loader.mjs': `export const peer = '${provider}';`,
        '.yarn/releases/yarn.cjs': `module.exports = '${provider}';`,
      },
    );
    const plan = migrate(root);
    expect(plan.changes).toEqual([]);
    expect(read(root).devDependencies[provider]).toBeUndefined();
  });
});

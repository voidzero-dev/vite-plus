import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { parse as parseJsonc } from 'jsonc-parser';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { parseDocument } from 'yaml';

import { PackageManager } from '../../types/index.ts';
import { applyVitestV5Migration, planVitestV5Migration } from '../migrator.ts';

vi.mock('yaml', async (importOriginal) => {
  const yaml = await importOriginal<typeof import('yaml')>();
  return { ...yaml, parseDocument: vi.fn(yaml.parseDocument) };
});

vi.mock('jsonc-parser', async (importOriginal) => {
  const jsonc = await importOriginal<typeof import('jsonc-parser')>();
  return { ...jsonc, parse: vi.fn(jsonc.parse) };
});

const directories: string[] = [];
afterEach(() => {
  vi.clearAllMocks();
  vi.restoreAllMocks();
  for (const directory of directories.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

const formats = ['pnpm', 'npm', 'npm-shrinkwrap', 'yarn', 'yarn-classic', 'bun'] as const;
function project(
  format: (typeof formats)[number],
  spec = '^4 || ^5',
  version = '4.1.11',
  lockedSpec = spec,
) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-vitest-lock-'));
  directories.push(root);
  fs.writeFileSync(
    path.join(root, 'package.json'),
    JSON.stringify({ devDependencies: { vitest: spec } }),
  );
  fs.writeFileSync(path.join(root, 'vite.config.ts'), 'export default { test: {} };');
  const manifest = { devDependencies: { vitest: lockedSpec } };
  let filename: string;
  let content: string;
  if (format === 'pnpm') {
    filename = 'pnpm-lock.yaml';
    content = JSON.stringify({
      lockfileVersion: '9.0',
      importers: {
        '.': { devDependencies: { vitest: { specifier: lockedSpec, version } } },
      },
    });
  } else if (format === 'npm' || format === 'npm-shrinkwrap') {
    filename = format === 'npm' ? 'package-lock.json' : 'npm-shrinkwrap.json';
    content = JSON.stringify({
      lockfileVersion: 3,
      packages: { '': manifest, 'node_modules/vitest': { version } },
    });
  } else if (format === 'bun') {
    filename = 'bun.lock';
    content = JSON.stringify({
      lockfileVersion: 1,
      workspaces: { '': manifest },
      packages: { vitest: [`vitest@${version}`, '', {}, 'sha512-test'] },
    });
  } else if (format === 'yarn') {
    filename = 'yarn.lock';
    content = `__metadata:\n  version: 8\n${JSON.stringify(`vitest@npm:${lockedSpec}`)}:\n  version: ${version}\n  resolution: "vitest@npm:${version}"\n`;
  } else {
    filename = 'yarn.lock';
    content = `# yarn lockfile v1\n\n${JSON.stringify(`vitest@${lockedSpec}`)}:\n  version "${version}"\n  resolved "https://registry.npmjs.org/vitest/-/vitest-${version}.tgz"\n`;
  }
  fs.writeFileSync(path.join(root, filename), content);
  const manager =
    format === 'yarn-classic'
      ? PackageManager.yarn
      : format === 'npm-shrinkwrap'
        ? PackageManager.npm
        : PackageManager[format];
  return {
    root,
    manager,
    file: path.join(root, filename),
    plan: () => planVitestV5Migration({ rootDir: root, packageManager: manager }),
  };
}

describe('Vitest source versions from lockfiles', () => {
  it.each(
    formats.flatMap((format) => ['4.1.11', '5.0.0'].map((version) => [format, version] as const)),
  )('uses the original %s lockfile runner %s without node_modules', (format, version) => {
    const fixture = project(format, '^4 || ^5', version);
    const plan = fixture.plan();
    expect(plan.projects[0].sourceVersion).toBe(version);
    expect(plan.projects[0].options.preserveV4).toBe(version.startsWith('4'));
    expect(plan.findings).toEqual([]);
    fs.appendFileSync(fixture.file, '\n');
    expect(() => applyVitestV5Migration(plan)).toThrow('Migration input changed');
  });

  it.each(formats.flatMap((format) => ['latest', '*'].map((spec) => [format, spec] as const)))(
    'resolves %s lockfile dist-tags and broad ranges: %s',
    (format, spec) => {
      expect(project(format, spec).plan().projects[0].sourceVersion).toBe('4.1.11');
    },
  );

  it.each(formats)('rejects stale %s importer/descriptor edges', (format) => {
    const plan = project(format, '^4 || ^5', '4.1.11', '^4.0.0').plan();
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'source-version', severity: 'block' }),
    );
  });

  it.each(formats)('rejects malformed %s locks', (format) => {
    const fixture = project(format);
    fs.writeFileSync(fixture.file, '{ invalid');
    expect(fixture.plan().projects[0].sourceVersion).toBeUndefined();
  });

  it('resolves a workspace npm edge before an unrelated root runner', () => {
    const fixture = project('npm');
    const pkg = { devDependencies: { vitest: '^4 || ^5' } };
    fs.mkdirSync(path.join(fixture.root, 'packages/unit'), { recursive: true });
    fs.writeFileSync(path.join(fixture.root, 'packages/unit/package.json'), JSON.stringify(pkg));
    fs.writeFileSync(
      fixture.file,
      JSON.stringify({
        lockfileVersion: 3,
        packages: {
          '': pkg,
          'packages/unit': pkg,
          'node_modules/vitest': { version: '5.0.0' },
          'packages/unit/node_modules/vitest': { version: '4.1.11' },
        },
      }),
    );
    const plan = planVitestV5Migration({
      rootDir: fixture.root,
      packageManager: PackageManager.npm,
      packages: [{ name: 'unit', path: 'packages/unit' }],
    });
    expect(plan.projects.map(({ sourceVersion }) => sourceVersion)).toEqual(['5.0.0', '4.1.11']);
  });

  it('reuses a pnpm lockfile within a plan without sharing versions between importers or plans', () => {
    const fixture = project('pnpm');
    const packages = [
      { name: 'unit', path: 'packages/unit' },
      { name: 'inherited', path: 'packages/inherited' },
    ];
    for (const pkg of packages) {
      fs.mkdirSync(path.join(fixture.root, pkg.path), { recursive: true });
      fs.writeFileSync(
        path.join(fixture.root, pkg.path, 'package.json'),
        JSON.stringify(pkg.name === 'unit' ? { devDependencies: { vitest: '^4 || ^5' } } : {}),
      );
    }
    const lock = JSON.parse(fs.readFileSync(fixture.file, 'utf8'));
    lock.importers['packages/unit'] = {
      devDependencies: { vitest: { specifier: '^4 || ^5', version: '5.0.0' } },
    };
    lock.importers['packages/inherited'] = {};
    fs.writeFileSync(fixture.file, JSON.stringify(lock));
    const workspace = { rootDir: fixture.root, packageManager: PackageManager.pnpm, packages };
    const plan = planVitestV5Migration(workspace);
    expect(plan.projects.map(({ sourceVersion }) => sourceVersion)).toEqual([
      '4.1.11',
      '5.0.0',
      '4.1.11',
    ]);
    expect(parseDocument).toHaveBeenCalledTimes(1);

    lock.importers['.'].devDependencies.vitest.version = '5.0.0';
    fs.writeFileSync(fixture.file, JSON.stringify(lock));
    expect(() => applyVitestV5Migration(plan)).toThrow('Migration input changed');
    expect(
      planVitestV5Migration(workspace).projects.map(({ sourceVersion }) => sourceVersion),
    ).toEqual(['5.0.0', '5.0.0', '5.0.0']);
    expect(parseDocument).toHaveBeenCalledTimes(2);

    fs.writeFileSync(fixture.file, '{ invalid');
    expect(
      planVitestV5Migration(workspace).projects.map(({ sourceVersion }) => sourceVersion),
    ).toEqual([undefined, undefined, undefined]);
    expect(parseDocument).toHaveBeenCalledTimes(3);
    fs.writeFileSync(fixture.file, JSON.stringify(lock));
    expect(
      planVitestV5Migration(workspace).projects.map(({ sourceVersion }) => sourceVersion),
    ).toEqual(['5.0.0', '5.0.0', '5.0.0']);
    expect(parseDocument).toHaveBeenCalledTimes(4);
  });

  it.each(formats)('reads and parses the %s lockfile once per plan', (format) => {
    const fixture = project(format);
    const member = 'packages/unit';
    fs.mkdirSync(path.join(fixture.root, member), { recursive: true });
    fs.writeFileSync(
      path.join(fixture.root, member, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '^4 || ^5' } }),
    );
    if (format === 'pnpm' || fixture.manager === PackageManager.npm || format === 'bun') {
      const lock = JSON.parse(fs.readFileSync(fixture.file, 'utf8'));
      const importers =
        format === 'pnpm'
          ? lock.importers
          : fixture.manager === PackageManager.npm
            ? lock.packages
            : lock.workspaces;
      importers[member] = importers[format === 'pnpm' ? '.' : ''];
      fs.writeFileSync(fixture.file, JSON.stringify(lock));
    }
    const before = fs.readFileSync(fixture.file, 'utf8');
    const reads = vi.spyOn(fs, 'readFileSync');
    const json = vi.spyOn(JSON, 'parse');
    const workspace = {
      rootDir: fixture.root,
      packageManager: fixture.manager,
      packages: [{ name: 'unit', path: member }],
    };
    const plan = planVitestV5Migration(workspace);
    expect(plan.projects.map(({ sourceVersion }) => sourceVersion)).toEqual(['4.1.11', '4.1.11']);
    // One read captures the input snapshot and one parses the lockfile.
    expect(reads.mock.calls.filter(([file]) => file === fixture.file)).toHaveLength(2);
    if (fixture.manager === PackageManager.npm) {
      expect(json.mock.calls.filter(([source]) => source === before)).toHaveLength(1);
    } else if (format === 'bun') {
      expect(vi.mocked(parseJsonc).mock.calls.filter(([source]) => source === before)).toHaveLength(
        1,
      );
    } else {
      expect(parseDocument).toHaveBeenCalledTimes(1);
    }

    const after = before.replaceAll('4.1.11', '5.0.0');
    fs.writeFileSync(fixture.file, after);
    expect(() => applyVitestV5Migration(plan)).toThrow('Migration input changed');
    expect(
      planVitestV5Migration(workspace).projects.map(({ sourceVersion }) => sourceVersion),
    ).toEqual(['5.0.0', '5.0.0']);

    fs.writeFileSync(fixture.file, '{ invalid');
    reads.mockClear();
    expect(
      planVitestV5Migration(workspace).projects.map(({ sourceVersion }) => sourceVersion),
    ).toEqual([undefined, undefined]);
    expect(reads.mock.calls.filter(([file]) => file === fixture.file)).toHaveLength(2);
    fs.writeFileSync(fixture.file, before);
    expect(
      planVitestV5Migration(workspace).projects.map(({ sourceVersion }) => sourceVersion),
    ).toEqual(['4.1.11', '4.1.11']);
  });

  it.each(['yarn', 'yarn-classic'] as const)(
    'keeps %s descriptor resolution package-specific when sharing a lockfile',
    (format) => {
      const fixture = project(format, '^4.1.0');
      fs.mkdirSync(path.join(fixture.root, 'unit'));
      fs.writeFileSync(
        path.join(fixture.root, 'unit/package.json'),
        JSON.stringify({ devDependencies: { vitest: '^5.0.0' } }),
      );
      fs.appendFileSync(
        fixture.file,
        format === 'yarn'
          ? '\n"vitest@npm:^5.0.0":\n  resolution: "vitest@npm:5.0.0"\n'
          : '\n"vitest@^5.0.0":\n  version "5.0.0"\n',
      );
      const plan = planVitestV5Migration({
        rootDir: fixture.root,
        packageManager: PackageManager.yarn,
        packages: [{ name: 'unit', path: 'unit' }],
      });
      expect(plan.projects.map(({ sourceVersion }) => sourceVersion)).toEqual(['4.1.11', '5.0.0']);
      expect(parseDocument).toHaveBeenCalledTimes(format === 'yarn' ? 1 : 2);
    },
  );

  it('prefers npm shrinkwrap over package-lock.json', () => {
    const fixture = project('npm-shrinkwrap');
    const otherLock = JSON.parse(fs.readFileSync(fixture.file, 'utf8'));
    otherLock.packages['node_modules/vitest'].version = '5.0.0';
    fs.writeFileSync(path.join(fixture.root, 'package-lock.json'), JSON.stringify(otherLock));
    expect(fixture.plan().projects[0].sourceVersion).toBe('4.1.11');
  });

  it('handles grouped Yarn classic descriptors', () => {
    const fixture = project('yarn-classic');
    fs.writeFileSync(
      fixture.file,
      `# yarn lockfile v1\n\n"vitest@^4 || ^5", vitest@^4.1.0:\n  version "4.1.11"\n`,
    );
    expect(fixture.plan().projects[0].sourceVersion).toBe('4.1.11');
  });

  it('rejects ambiguous Bun layouts instead of choosing a transitive version', () => {
    const fixture = project('bun');
    const lock = JSON.parse(fs.readFileSync(fixture.file, 'utf8'));
    lock.packages['other/vitest'] = ['vitest@5.0.0', '', {}, 'sha512-test'];
    fs.writeFileSync(fixture.file, JSON.stringify(lock));
    expect(fixture.plan().projects[0].sourceVersion).toBeUndefined();
  });

  it('does not use an unrelated nested Bun runner as the direct dependency', () => {
    const fixture = project('bun');
    const lock = JSON.parse(fs.readFileSync(fixture.file, 'utf8'));
    lock.packages['other/vitest'] = lock.packages.vitest;
    delete lock.packages.vitest;
    fs.writeFileSync(fixture.file, JSON.stringify(lock));
    expect(fixture.plan().projects[0].sourceVersion).toBeUndefined();
  });
});

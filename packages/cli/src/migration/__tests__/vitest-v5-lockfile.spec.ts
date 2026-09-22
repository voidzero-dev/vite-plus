import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import { applyVitestV5Migration, planVitestV5Migration } from '../migrator.ts';

const directories: string[] = [];
afterEach(() => {
  for (const directory of directories.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

const formats = ['npm', 'yarn', 'yarn-classic', 'bun'] as const;
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
  if (format === 'npm') {
    filename = 'package-lock.json';
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
  const manager = format === 'yarn-classic' ? PackageManager.yarn : PackageManager[format];
  return {
    root,
    file: path.join(root, filename),
    plan: () => planVitestV5Migration({ rootDir: root, packageManager: manager }),
  };
}

describe('Vitest source versions from non-pnpm lockfiles', () => {
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

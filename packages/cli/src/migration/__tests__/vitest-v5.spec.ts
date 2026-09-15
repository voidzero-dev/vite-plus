import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { runInNewContext } from 'node:vm';

import { afterEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  applyVitestV5Migration,
  applyVitestV5NodeMigration,
  createVitestV5CompatibilityConfig,
  finishVitestV5Migration,
  formatVitestV5Findings,
  planVitestV5Migration,
  refreshVitestV5Migration,
  vitestV5ConfiglessProjects,
  vitestV5NeedsMigration,
} from '../migrator.ts';
import { parseSource } from '../vitest-v5/ast.ts';
import { migrateVitestV5Command } from '../vitest-v5/commands.ts';
import { migrateVitestV5Config } from '../vitest-v5/config.ts';
import { migrateVitestV5Source } from '../vitest-v5/source.ts';

const v4 = { preserveV4: true, browser: true };
const directories: string[] = [];
afterEach(() => {
  for (const directory of directories.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
function project(files: Record<string, string> = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-vitest-v5-'));
  directories.push(root);
  const all = {
    'package.json': JSON.stringify({ devDependencies: { vitest: '^4.1.0' } }),
    ...files,
  };
  for (const [name, content] of Object.entries(all)) {
    const file = path.join(root, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, content);
  }
  return root;
}
const planProject = (rootDir: string) =>
  planVitestV5Migration({ rootDir, packageManager: PackageManager.pnpm });
function config(source: string, preserveV4 = true) {
  const result = migrateVitestV5Config('vite.config.ts', source, { preserveV4 });
  parseSource('vite.config.ts', result.content);
  return result;
}
function source(input: string, options = v4) {
  const result = migrateVitestV5Source('example.test.ts', input, options);
  parseSource('example.test.ts', result.content);
  return result;
}

describe('Vitest v5 config compatibility', () => {
  it('retains benchmark JSON consumer reviews through finalization, without repeat edits', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { bench: 'vitest bench --outputJson=cli.json' },
      }),
      'vitest.config.mjs': `export default { test: { benchmark: { outputJson: 'config.json' } } };`,
      'example.bench.js': `import { bench } from 'vitest'; const work = () => 42; bench('work', work);`,
    });
    const plan = planProject(root);
    expect(plan.findings.map(({ code }) => code)).toEqual(['benchmark-output', 'benchmark-output']);
    applyVitestV5Migration(plan);
    const findings = finishVitestV5Migration(plan);
    expect(findings.map(({ code }) => code)).toEqual(['benchmark-output', 'benchmark-output']);
    expect(formatVitestV5Findings({ rootDir: root, findings })).toContain(
      'Docs: https://vitest.dev/guide/migration/#benchmarking-api-rewrite',
    );
    expect(planProject(root).changes).toEqual([]);
    expect(planProject(root).findings).toEqual([]);
  });

  it.each(['mjs', 'ts'])('selects vitest.config.%s over an inactive Vite config', (extension) => {
    const root = project({
      'vite.config.ts': 'export default {};',
      [`vitest.config.${extension}`]: 'export default { test: { globals: true } };',
      'unit.test.js': `test.sequential('works', () => { expect(() => {}).toThrow(''); });`,
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    const migrated = fs.readFileSync(path.join(root, 'unit.test.js'), 'utf8');
    expect(migrated).toContain("test('works', { concurrent: false }");
    expect(migrated).toContain('toThrow(/^$/)');
    expect(planProject(root).changes).toEqual([]);
  });

  it('migrates global benchmarks despite an inactive Vite config', () => {
    const root = project({
      'vite.config.mjs': 'export default {};',
      'vitest.config.mjs': 'export default { test: { globals: true } };',
      'unit.test.js': `bench('old', () => {});`,
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    expect(plan.changes.find(({ file }) => file.endsWith('unit.test.js'))?.after).toContain(
      'globalThis.test',
    );
  });

  it('resolves an inline test.root relative to the declaring project root', () => {
    const root = project({
      'vitest.config.mjs': `export default { root: './parent', test: { projects: [{ extends: false, root: './app', test: { root: './unit', globals: true } }] } };`,
      'parent/unit/right.test.js': `test.sequential('works', () => {});`,
      'parent/app/unit/wrong.test.js': `expect(() => {}).toThrow('');`,
      'unit/wrong.test.js': `expect(() => {}).toThrow('');`,
    });
    const plan = planProject(root);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'parent/unit/right.test.js'))?.after,
    ).toContain('concurrent: false');
    expect(plan.changes.filter(({ file }) => file.endsWith('wrong.test.js'))).toEqual([]);
  });

  it('uses an explicit config path relative to the script root, not its own directory', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { test: 'vitest --config configs/custom.mjs' },
      }),
      'configs/custom.mjs': `export default { root: './app', test: { root: './unit', globals: true, setupFiles: './setup.js' } };`,
      'unit/right.test.js': `test.sequential('works', () => {});`,
      'unit/setup.js': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
      'configs/unit/wrong.test.js': `expect(() => {}).toThrow('');`,
    });
    const plan = planProject(root);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'unit/right.test.js'))?.after,
    ).toContain('concurrent: false');
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'unit/setup.js'))?.after,
    ).toContain('beforeEach(async');
    expect(plan.changes.some(({ file }) => file.endsWith('wrong.test.js'))).toBe(false);
  });

  it('does not apply a CLI root again to inline project roots', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { test: 'vitest --root parent' },
      }),
      'parent/vitest.config.mjs': `export default { test: { projects: [{ test: { root: './unit', globals: true } }] } };`,
      'parent/unit/right.test.js': `test.sequential('works', () => {});`,
      'parent/wrong.test.js': `expect(() => {}).toThrow('');`,
    });
    const plan = planProject(root);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'parent/unit/right.test.js'))?.after,
    ).toContain('concurrent: false');
    expect(plan.changes.some(({ file }) => file.endsWith('wrong.test.js'))).toBe(false);
  });

  it.each([
    'vitest --config custom.mjs',
    'vitest -c custom.mjs',
    'vp test --config=custom.mjs',
    'pnpm exec vitest run --config "custom.mjs"',
  ])('respects explicit config selection: %s', (command) => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { test: command },
      }),
      'vitest.config.mjs': 'export default { test: { globals: false } };',
      'custom.mjs': 'export default { test: { globals: true } };',
      'unit.test.js': `test.sequential('works', () => {});`,
    });
    const plan = planProject(root);
    expect(plan.projects[0].configFiles).toContain(path.join(root, 'custom.mjs'));
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'unit.test.js'))?.after,
    ).toContain('concurrent: false');
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    expect(planProject(root).changes).toEqual([]);
  });

  it.each([
    `export default { root: './app', test: { root: './unit', globals: true } };`,
    `export default { root: rootAtRuntime, test: { root: './unit', globals: true } };`,
    `export default { test: { projects: [{ root: './app', test: { root: './unit', globals: true } }] } };`,
    `export default { test: { globals: true, projects: [{ extends: true, root: './app', test: { root: './unit' } }] } };`,
  ])('uses test.root instead of concatenating roots: %s', (configSource) => {
    const unrelated = `expect(() => { throw new Error('boom'); }).toThrow('');`;
    const root = project({
      'vitest.config.mjs': configSource,
      'unit/right.test.js': `test.sequential('works', () => {});`,
      'app/unit/wrong.test.js': unrelated,
    });
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'unit/right.test.js'), 'utf8')).toContain(
      'concurrent: false',
    );
    expect(fs.readFileSync(path.join(root, 'app/unit/wrong.test.js'), 'utf8')).toBe(unrelated);
    expect(plan.findings).toEqual([]);
  });

  it.each([
    `export default { test: { globals: true, dir: './unit', include: ['*.test.js'], exclude: ['excluded.test.js'], setupFiles: './setup.js' } };`,
    `export default { root: './app', test: { globals: true, dir: './unit', include: ['*.test.js'], exclude: ['excluded.test.js'], setupFiles: '../setup.js' } };`,
    `export default { test: { projects: [{ extends: false, test: { globals: true, dir: './unit', include: ['*.test.js'], exclude: ['excluded.test.js'], setupFiles: './setup.js' } }] } };`,
    `export default { test: { globals: true, dir: './unit', include: ['*.test.js'], exclude: ['excluded.test.js'], setupFiles: './setup.js', projects: [{ extends: true, test: {} }] } };`,
  ])('uses test.dir for discovery without changing setup resolution: %s', (configSource) => {
    const unrelated = `expect(() => { throw new Error('boom'); }).toThrow('');`;
    const root = project({
      'vitest.config.mjs': configSource,
      'unit/right.test.js': `test.sequential('works', () => {});`,
      'wrong.test.js': unrelated,
      'unit/excluded.test.js': unrelated,
      'app/unit/wrong.test.js': unrelated,
      'setup.js': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
      'unit/setup.js': unrelated,
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'unit/right.test.js'), 'utf8')).toContain(
      'concurrent: false',
    );
    expect(fs.readFileSync(path.join(root, 'setup.js'), 'utf8')).toContain(
      'beforeEach(async () => { await expect',
    );
    for (const file of [
      'wrong.test.js',
      'unit/excluded.test.js',
      'app/unit/wrong.test.js',
      'unit/setup.js',
    ]) {
      expect(fs.readFileSync(path.join(root, file), 'utf8')).toBe(unrelated);
    }
    finishVitestV5Migration(plan);
    expect(planProject(root).findings).toEqual([]);
    expect(planProject(root).changes).toEqual([]);
  });

  it.each(['vitest run --dir ./unit', 'vitest --dir=./unit', 'vp test --dir ./unit'])(
    'honors a literal discovery directory override: %s',
    (command) => {
      const unrelated = `expect(() => {}).toThrow('');`;
      const root = project({
        'package.json': JSON.stringify({
          devDependencies: { vitest: '4.1.11' },
          scripts: { test: command },
        }),
        'vitest.config.mjs': `export default { test: { globals: true, dir: './ignored', include: ['*.test.js'] } };`,
        'unit/right.test.js': `test.sequential('works', () => {});`,
        'wrong.test.js': unrelated,
        'ignored/wrong.test.js': unrelated,
      });
      const plan = planProject(root);
      expect(plan.findings).toEqual([]);
      applyVitestV5Migration(plan);
      expect(fs.readFileSync(path.join(root, 'unit/right.test.js'), 'utf8')).toContain(
        'concurrent: false',
      );
      for (const file of ['wrong.test.js', 'ignored/wrong.test.js']) {
        expect(fs.readFileSync(path.join(root, file), 'utf8')).toBe(unrelated);
      }
    },
  );

  it.each<{ projects: string; files: Record<string, string> }>([
    { projects: `[{ extends: true, test: {} }]`, files: {} },
    {
      projects: `['./configs/project.mjs']`,
      files: {
        'configs/project.mjs': `export default { test: { globals: true, dir: './unit', include: ['*.test.js'] } };`,
      },
    },
  ])('does not forward --dir to declared projects: $projects', ({ projects, files }) => {
    const unrelated = `expect(() => {}).toThrow('');`;
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { test: 'vitest --dir ./ignored' },
      }),
      'vitest.config.mjs': `export default { test: { globals: true, dir: './unit', include: ['*.test.js'], projects: ${projects} } };`,
      'unit/right.test.js': `test.sequential('works', () => {});`,
      'ignored/wrong.test.js': unrelated,
      'configs/unit/wrong.test.js': unrelated,
      ...files,
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'unit/right.test.js'), 'utf8')).toContain(
      'concurrent: false',
    );
    expect(fs.readFileSync(path.join(root, 'ignored/wrong.test.js'), 'utf8')).toBe(unrelated);
    expect(fs.readFileSync(path.join(root, 'configs/unit/wrong.test.js'), 'utf8')).toBe(unrelated);
  });

  it('reviews an unresolved discovery directory while migrating known setup files', () => {
    const input = `test.sequential('works', () => {});`;
    const root = project({
      'vitest.config.mjs': `export default { test: { globals: true, dir: directoryAtRuntime, setupFiles: './setup.js' } };`,
      'unit/right.test.js': input,
      'wrong.test.js': input,
      'setup.js': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
    });
    const plan = planProject(root);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        file: path.join(root, 'vitest.config.mjs'),
        code: 'global-api-ownership',
      }),
    );
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'unit/right.test.js'), 'utf8')).toBe(input);
    expect(fs.readFileSync(path.join(root, 'wrong.test.js'), 'utf8')).toBe(input);
    expect(fs.readFileSync(path.join(root, 'setup.js'), 'utf8')).toContain(
      'beforeEach(async () => { await expect',
    );
    finishVitestV5Migration(plan);
    expect(planProject(root).findings).toContainEqual(
      expect.objectContaining({ code: 'global-api-ownership' }),
    );
  });

  it.each([
    ['setup.mjs', 'setup.cjs', 'setup.js'],
    ['setup.cjs', 'setup.js'],
    ['setup.js', 'setup/index.mjs'],
    ['setup/index.mjs', 'setup/index.cjs', 'setup/index.js'],
    ['setup/index.cjs', 'setup/index.js'],
    ['setup/index.js'],
  ])('uses Vitest setup resolution precedence: %j', (...files) => {
    const input = 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });';
    const unrelated = `expect(() => { throw new Error('boom'); }).toThrow('');`;
    const root = project({
      'vitest.config.mjs': `export default { test: { globals: true, setupFiles: ['./setup'] } };`,
      'unit.test.js': `test('works', () => {});`,
      ...Object.fromEntries(files.map((file, index) => [file, index === 0 ? input : unrelated])),
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, files[0]), 'utf8')).toContain(
      'beforeEach(async () => { await expect',
    );
    for (const file of files.slice(1)) {
      expect(fs.readFileSync(path.join(root, file), 'utf8')).toBe(unrelated);
    }
    finishVitestV5Migration(plan);
    expect(planProject(root).findings).toEqual([]);
    expect(planProject(root).changes).toEqual([]);
  });

  it('keeps a trailing-slash setup directory distinct from a sibling module', () => {
    const unrelated = `expect(() => {}).toThrow('');`;
    const root = project({
      'vitest.config.mjs': `export default { test: { globals: true, setupFiles: ['./setup/'] } };`,
      'setup.mjs': unrelated,
      'setup/index.js': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'setup.mjs'), 'utf8')).toBe(unrelated);
    expect(fs.readFileSync(path.join(root, 'setup/index.js'), 'utf8')).toContain(
      'beforeEach(async () => { await expect',
    );
  });

  it.each(['./missing', './setup', 'setup-package'])(
    'reviews unresolved setup entry %s even without global calls',
    (setup) => {
      const root = project({
        'vitest.config.mjs': `export default { test: { globals: true, setupFiles: ['${setup}'] } };`,
        'unit.test.js': `import { test } from 'vitest'; test('works', () => {});`,
        'setup.ts': `export const unrelated = true;`,
      });
      const plan = planProject(root);
      expect(plan.findings).toContainEqual(
        expect.objectContaining({
          file: path.join(root, 'vitest.config.mjs'),
          code: 'global-api-ownership',
        }),
      );
      applyVitestV5Migration(plan);
      finishVitestV5Migration(plan);
      expect(planProject(root).findings).toContainEqual(
        expect.objectContaining({ code: 'global-api-ownership' }),
      );
    },
  );

  it.each([
    `export default { test: { globals: true, setupFiles: './setup.js', include: ['unit.test.js'], exclude: ['setup.js'] } };`,
    `export default { test: { globals: true, setupFiles: ['./setup.js'] } };`,
    `export default { test: { globals: true, setupFiles: ['./setup.js'], projects: [{ extends: true, test: {} }] } };`,
    `export default { test: { projects: [{ test: { globals: true, setupFiles: ['./setup.js'] } }] } };`,
  ])('migrates globals in declared setup files: %s', (configSource) => {
    const root = project({
      'vitest.config.mjs': configSource,
      'setup.js': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
      'unit.test.js': `test('works', () => {});`,
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'setup.js'), 'utf8')).toBe(
      'beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });',
    );
    expect(planProject(root).changes).toEqual([]);
  });

  it('applies project directory precedence and setup ownership to referenced configs', () => {
    const root = project({
      'vitest.config.mjs': `export default { test: { projects: ['./unit'] } };`,
      'unit/vite.config.ts': 'export default {};',
      'unit/vitest.config.mjs': `export default { test: { globals: true, setupFiles: '../shared/setup.js' } };`,
      'unit/right.test.js': `test.sequential('works', () => {});`,
      'shared/setup.js': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'unit/right.test.js'))?.after,
    ).toContain('concurrent: false');
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'shared/setup.js'))?.after,
    ).toContain('beforeEach(async () => { await expect');
  });

  it.each([
    `export default { test: { projects: [{ test: { globals: true, setupFiles: './setup.js' } }, { test: { globals: false, setupFiles: './setup.js' } }] } };`,
    `export default { test: { globals: true, setupFiles: filesAtRuntime } };`,
    `export default () => ({ test: { globals: true, setupFiles: './setup.js' } });`,
  ])('reports unresolved global ownership without rewriting calls: %s', (configSource) => {
    const input = 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });';
    const root = project({
      'vitest.config.mjs': configSource,
      'setup.js': input,
      'imported.js': `import { expect } from 'vitest'; expect(() => {}).toThrow('');`,
    });
    const plan = planProject(root);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'global-api-ownership', file: path.join(root, 'setup.js') }),
    );
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'setup.js'), 'utf8')).toBe(input);
    expect(fs.readFileSync(path.join(root, 'imported.js'), 'utf8')).toContain('toThrow(/^$/)');
    expect(planProject(root).findings).toContainEqual(
      expect.objectContaining({ code: 'global-api-ownership' }),
    );
  });

  it.each([
    'vitest --config "$CONFIG"',
    'cd unit && vitest',
    'vitest --config missing.mjs',
    'vitest --dir "$TEST_DIR"',
    'vitest --dir=$TEST_DIR',
    'vitest --dir',
  ])('reports unresolved script selections: %s', (command) => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { test: command },
      }),
      'vitest.config.mjs': 'export default { test: { globals: true } };',
      'unit.test.js': `test.sequential('works', () => {});`,
    });
    const plan = planProject(root);
    expect(plan.changes.some(({ file }) => file === path.join(root, 'unit.test.js'))).toBe(false);
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
  });

  it.each([`{ name: 'unit' }`, `{ name: 'unit', browser: { enabled: true } }`])(
    'preserves defaults inherited from an external base: %s',
    (testOptions) => {
      const child = `{ extends: './base.mjs', test: ${testOptions} }`;
      const base = `export default { test: { clearMocks: true, browser: { locators: { exact: true } } } };`;
      const root = project({
        'base.mjs': base,
        'vitest.config.mjs': `export default { test: { projects: [${child}] } };`,
      });
      const plan = planProject(root);
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'project-inheritance', severity: 'review' }),
      );
      applyVitestV5Migration(plan);
      finishVitestV5Migration(plan);
      expect(fs.readFileSync(path.join(root, 'base.mjs'), 'utf8')).toBe(base);
      expect(fs.readFileSync(path.join(root, 'vitest.config.mjs'), 'utf8')).toContain(child);
      // Without upgrading the runner, the v4 inheritance review still applies.
      const repeated = planProject(root);
      expect(repeated.changes).toEqual([]);
      expect(repeated.findings).toContainEqual(
        expect.objectContaining({ code: 'project-inheritance' }),
      );
      fs.writeFileSync(
        path.join(root, 'package.json'),
        JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
      );
      const upgraded = planProject(root);
      expect(upgraded.changes).toEqual([]);
      expect(upgraded.findings.some(({ code }) => code === 'project-inheritance')).toBe(false);
    },
  );

  it.each([`'./missing.mjs'`, `baseConfig`])(
    'preserves unresolved project inheritance: %s',
    (base) => {
      const child = `{ extends: ${base} }`;
      const result = config(`export default { test: { projects: [${child}] } };`);
      expect(result.content).toContain(child);
      expect(result.findings).toContainEqual(
        expect.objectContaining({ code: 'project-inheritance' }),
      );
    },
  );

  it('limits Vitest globals to included files in a mixed Jest/Vitest package', () => {
    const input = `test('throws', () => { expect(() => { throw new Error('boom'); }).toThrow(''); });`;
    const root = project({
      'package.json': JSON.stringify({ devDependencies: { vitest: '4.1.11', jest: '29.7.0' } }),
      'vitest.config.mjs': `export default { test: { globals: true, include: ['unit/**/*.test.ts', 'unit/custom.ts'], exclude: ['unit/excluded.test.ts'] } };`,
      'jest.config.cjs': `module.exports = { testMatch: ['<rootDir>/integration/**/*.test.ts'] };`,
      'unit/example.test.ts': input,
      'unit/custom.ts': input,
      'unit/excluded.test.ts': input,
      'integration/other.test.ts': input,
      'helpers/imported.test.ts': `import { test, expect } from 'vitest';\n${input}`,
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    for (const name of ['unit/example.test.ts', 'unit/custom.ts', 'helpers/imported.test.ts']) {
      expect(fs.readFileSync(path.join(root, name), 'utf8')).toContain('toThrow(/^$/)');
    }
    for (const name of ['unit/excluded.test.ts', 'integration/other.test.ts']) {
      expect(fs.readFileSync(path.join(root, name), 'utf8')).toBe(input);
    }
    expect(planProject(root).changes).toEqual([]);
  });

  it.each([
    `import { defineConfig } from 'vitest/config'; const unused = defineConfig({ test: { globals: true } }); export default { test: { include: ['other.test.ts'] } };`,
    `export default { test: { include: ['unit.test.ts'] } }; // globals: true`,
    `export default { test: { globals: enabledAtRuntime } };`,
    `export default { test: { globals: true, include: patterns } };`,
    `export default { test: { globals: true, exclude: patterns } };`,
    `export default () => ({ test: { globals: true } });`,
    `export default { test: { globals: true, projects: [{ test: { include: ['unit.test.ts'] } }] } };`,
    `export default { test: { projects: [{ test: { globals: true } }, { test: { globals: false } }] } };`,
  ])('preserves unbound APIs without certain Vitest global ownership: %s', (configSource) => {
    const input = `test.sequential('x', () => expect(() => {}).toThrow(''));`;
    const root = project({ 'vitest.config.mjs': configSource, 'unit.test.ts': input });
    expect(
      planProject(root).changes.some(({ file }) => file === path.join(root, 'unit.test.ts')),
    ).toBe(false);
  });

  it.each([
    `import { defineConfig } from 'vitest/config'; export default defineConfig({ test: { globals: true, include: ['unit/**/*.test.ts'] } });`,
    `import { defineConfig } from 'vitest/config'; const config = defineConfig({ test: { globals: true, include: ['unit/**/*.test.ts'] } }); export default config;`,
    `import { defineConfig } from 'vitest/config'; module.exports = defineConfig({ test: { globals: true, include: ['unit/**/*.test.ts'] } });`,
    `export default { test: { projects: [{ test: { globals: true, include: ['unit/**/*.test.ts'] } }, { test: { globals: false, include: ['integration/**/*.test.ts'] } }] } };`,
    `export default { test: { globals: true, include: ['unit/**/*.test.ts'], projects: [{ extends: true, test: {} }, { extends: false, test: { include: ['integration/**/*.test.ts'] } }] } };`,
    `export default { root: './unit', test: { globals: true } };`,
  ])('resolves global ownership from inline projects and roots: %s', (configSource) => {
    const input = `expect(() => {}).toThrow('');`;
    const root = project({
      'vitest.config.mjs': configSource,
      'unit/example.test.ts': input,
      'integration/other.test.ts': input,
    });
    const plan = planProject(root);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'unit/example.test.ts'))?.after,
    ).toContain('toThrow(/^$/)');
    expect(
      plan.changes.some(({ file }) => file === path.join(root, 'integration/other.test.ts')),
    ).toBe(false);
  });

  it('resolves global ownership from a referenced project config', () => {
    const input = `expect(() => {}).toThrow('');`;
    const root = project({
      'vitest.config.mjs': `export default { test: { projects: ['./unit/project.mjs'] } };`,
      'unit/project.mjs': `export default { test: { globals: true } };`,
      'unit/example.test.ts': input,
      'integration/other.test.ts': input,
    });
    const plan = planProject(root);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'unit/example.test.ts'))?.after,
    ).toContain('toThrow(/^$/)');
    expect(
      plan.changes.some(({ file }) => file === path.join(root, 'integration/other.test.ts')),
    ).toBe(false);
  });

  it.each(['vite', 'vite-plus'])(
    'discovers custom test configs that import helpers from %s',
    (module) => {
      const root = project({
        'package.json': JSON.stringify({
          devDependencies: { vitest: '^4.1.0' },
          scripts: { test: 'vitest --config custom.ts' },
        }),
        'custom.ts': `import { defineConfig } from '${module}'; export default defineConfig({ test: { browser: { enabled: true } } });`,
      });
      const plan = planProject(root);
      expect(plan.projects[0].configFiles).toContain(path.join(root, 'custom.ts'));
      expect(
        plan.changes.find(({ file }) => file === path.join(root, 'custom.ts'))?.after,
      ).toContain('exact: false');
      expect(plan.findings).toEqual([]);
    },
  );

  it.each(['vite', 'vite-plus'])(
    'does not treat a source example using %s helpers as a test config',
    (module) => {
      const input = `import { defineConfig, type Plugin } from '${module}';
import { describe, it, expect } from 'vitest';
const unrelated = { test: {} };
function plugin(): Plugin { return { name: 'example', configResolved() {} }; }
describe('plugin', () => { it('works', () => { expect(plugin()).toBeDefined(); }); });
export default defineConfig({ plugins: [plugin()] });`;
      const root = project({ 'src/index.ts': input });
      const plan = planProject(root);
      expect(plan.projects[0].configFiles.size).toBe(0);
      expect(plan.changes).toEqual([]);
      expect(plan.findings.map(({ code }) => code)).toEqual(['configless-defaults']);
      applyVitestV5Migration(plan);
      finishVitestV5Migration(plan);
      expect(fs.readFileSync(path.join(root, 'src/index.ts'), 'utf8')).toBe(input);
      expect(planProject(root).projects[0].configFiles.size).toBe(0);
    },
  );

  it.each(['vite', 'vite-plus'])(
    'preserves defaults in named and referenced %s configs without test options',
    (module) => {
      const root = project({
        'vite.config.ts': `import { defineConfig } from '${module}'; export default defineConfig({});`,
        'vitest.config.ts': `export default { test: { projects: ['./custom.ts'] } };`,
        'custom.ts': `import { defineConfig } from '${module}'; export default defineConfig({});`,
      });
      const plan = planProject(root);
      expect(plan.projects[0].configFiles.size).toBe(3);
      for (const name of ['vite.config.ts', 'custom.ts']) {
        expect(plan.changes.find(({ file }) => file === path.join(root, name))?.after).toContain(
          'clearMocks: false',
        );
      }
    },
  );

  it.each([
    `export default wrapper(defineConfig({ test: { browser: { enabled: true } } }));`,
    `module.exports = wrapper(defineConfig({ test: { browser: { enabled: true } } }));`,
    `const fragment = defineConfig({ test: {} }); export default wrapper(fragment);`,
  ])('leaves wrapped config defaults unchanged: %s', (declaration) => {
    const input = `import { defineConfig } from 'vitest/config'; ${declaration}`;
    const result = config(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(expect.objectContaining({ code: 'dynamic-config' }));
    const root = project({ 'vite.config.ts': input });
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toBe(input);
  });

  it('does not add defaults to a dynamic inline defineProject fragment', () => {
    const fragment = 'defineProject({ test: { browser: { enabled: true } } })';
    const result = config(
      `import { defineConfig, defineProject } from 'vitest/config'; export default defineConfig({ test: { projects: [${fragment}] } });`,
    );
    expect(result.content).toContain(fragment);
    expect(result.findings).toContainEqual(expect.objectContaining({ code: 'dynamic-project' }));
  });

  it.each([`'html'`, `['html', {}]`, `['html', { outputDir: 'reports' }]`])(
    'reports top-level HTML outputFile with reporter %s',
    (reporter) => {
      expect(
        config(
          `export default { test: { reporters: [${reporter}], outputFile: 'old/index.html' } };`,
        ).findings,
      ).toContainEqual(expect.objectContaining({ code: 'html-output' }));
    },
  );
  it('keeps Node jest-dom matchers separate from browser matchers in one package', () => {
    const nodeTest = `import { expect } from 'vitest';\nexpect(element).toHaveTextContent('partial');`;
    const root = project({
      'vite.config.ts': `export default { test: { projects: [
        { test: { environment: 'jsdom', include: ['node.test.ts'] } },
        { test: { browser: { enabled: true }, include: ['browser.test.ts'] } },
      ] } };`,
      'node.test.ts': nodeTest,
      'browser.test.ts': nodeTest,
    });
    const plan = planProject(root);
    expect(plan.changes.some(({ file }) => file === path.join(root, 'node.test.ts'))).toBe(false);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'browser.test.ts'))?.after,
    ).toContain("expect(element).toMatchTextContent('partial')");
    expect(plan.findings.some(({ code }) => code === 'text-content-project')).toBe(false);
  });

  it.each([
    `export default { test: { projects: [
      { test: { include: ['shared.test.ts'] } },
      { test: { browser: { enabled: true }, include: ['shared.test.ts'] } },
    ] } };`,
    `export default () => ({ test: { browser: { enabled: true } } });`,
    `import { defineConfig } from 'vitest/config';
export default wrapper(defineConfig({ test: { browser: { enabled: true } } }));`,
    `export default { test: { projects: ['unknown-config.ts',
      { test: { browser: { enabled: true } } },
    ] } };`,
    `export default { test: { browser: { enabled: enabledAtRuntime },
      projects: [{ extends: true, test: {} }],
    } };`,
    `export default { test: { browser: { enabled: true },
      include: ['**/*.test.ts', '!shared.test.ts'],
    } };`,
    `import { defineConfig } from 'vitest/config';
module.exports = wrapper(defineConfig({ test: { browser: { enabled: true } } }));`,
  ])(
    'reports ambiguous browser ownership without changing a Node-compatible matcher: %s',
    (configuration) => {
      const root = project({
        'vite.config.ts': configuration,
        'shared.test.ts': `import { expect } from 'vitest';\nexpect(element).toHaveTextContent('partial');`,
      });
      const plan = planProject(root);
      expect(plan.changes.some(({ file }) => file === path.join(root, 'shared.test.ts'))).toBe(
        false,
      );
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'text-content-project', severity: 'review' }),
      );
    },
  );

  it('renames expect.element matchers even when project ownership is dynamic', () => {
    const root = project({
      'vite.config.ts': `export default () => ({ test: { browser: { enabled: true } } });`,
      'browser.test.ts': `import { expect } from 'vitest';\nawait expect.element(element).toHaveTextContent('partial');`,
    });
    const plan = planProject(root);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'browser.test.ts'))?.after,
    ).toContain("expect.element(element).toMatchTextContent('partial')");
  });

  it('resolves a named config helper and referenced raw project configs', () => {
    const input = `import { expect } from 'vitest';\nexpect(element).toHaveTextContent('partial');`;
    const root = project({
      'vite.config.ts': `import { defineConfig } from 'vitest/config';
const config = defineConfig({ test: { projects: ['./node-project.ts', './browser-project.ts'] } });
export default config;`,
      'node-project.ts': `export default { test: { include: ['node.test.ts'] } };`,
      'browser-project.ts': `export default { test: { browser: { enabled: true }, include: ['browser.test.ts'] } };`,
      'node.test.ts': input,
      'browser.test.ts': input,
    });
    const plan = planProject(root);
    expect(plan.changes.some(({ file }) => file === path.join(root, 'node.test.ts'))).toBe(false);
    expect(
      plan.changes.find(({ file }) => file === path.join(root, 'browser.test.ts'))?.after,
    ).toContain('toMatchTextContent');
  });

  it('does not infer helper-module ownership from default test globs', () => {
    const root = project({
      'vite.config.ts': 'export default { test: { browser: { enabled: true } } };',
      'assertions.ts': `import { expect } from 'vitest';\nexport function check(element) { expect(element).toHaveTextContent('partial'); }`,
    });
    const plan = planProject(root);
    expect(plan.changes.some(({ file }) => file === path.join(root, 'assertions.ts'))).toBe(false);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'text-content-project', severity: 'review' }),
    );
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'text-content-project', severity: 'review' }),
    );
    expect(planProject(root).findings).toContainEqual(
      expect.objectContaining({ code: 'text-content-project', severity: 'review' }),
    );
  });

  it('does not infer a Node-only assertion when scripts override browser mode', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { test: 'vitest', 'test:browser': 'vitest --browser' },
      }),
      'vite.config.ts': 'export default { test: {} };',
      'shared.test.ts': `import { expect } from 'vitest';\nexpect(element).toHaveTextContent('partial');`,
    });
    const plan = planProject(root);
    expect(plan.changes.some(({ file }) => file === path.join(root, 'shared.test.ts'))).toBe(false);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'text-content-project', severity: 'review' }),
    );
  });

  it('retains multiline indentation and comments when adding defaults', () => {
    const result = config(
      'export default {\r\n  test: {\r\n    // Keep this comment.\r\n    globals: true,\r\n  },\r\n};',
    );
    expect(result.content).toBe(
      'export default {\r\n  test: {\r\n    clearMocks: false,\r\n    // Keep this comment.\r\n    globals: true,\r\n  },\r\n};',
    );
    expect(config(result.content).content).toBe(result.content);
  });
  it('preserves raw CommonJS config defaults', () => {
    const result = migrateVitestV5Config('vitest.config.cjs', 'module.exports = { test: {} };', v4);
    expect(result.content).toContain('clearMocks: false');
    expect(result.findings).toEqual([]);
  });
  it('preserves root and non-inheriting project defaults without overriding explicit choices', () => {
    const result = config(`import { defineConfig } from 'vite-plus';
export default defineConfig({ test: { projects: [
  { test: { browser: { enabled: true } } },
  { extends: true, test: {} },
  { extends: './base.ts', test: { clearMocks: true, browser: { locators: { exact: true } } } },
] } });`);
    expect(result.content).toContain('sharedViteServer: false');
    expect(result.content.match(/clearMocks: false/g)).toHaveLength(2);
    expect(result.content).toContain('clearMocks: true');
    expect(result.content).toContain('exact: false');
    expect(result.content).toContain('exact: true');
    expect(result.content.match(/extends: false/g)).toHaveLength(1);
    expect(config(result.content).content).toBe(result.content);
  });

  it('does not apply v4 defaults to v5 configs', () => {
    const input = `export default { test: { projects: [{ test: {} }] } };`;
    expect(config(input, false).content).toBe(input);
  });

  it.each([
    `export default mergeConfig(
      defineConfig({ test: { clearMocks: true } }),
      defineConfig({ test: { environment: 'jsdom' } }),
    );`,
    `const base = defineConfig({ test: { clearMocks: true } });
const overrides = defineConfig({ test: { environment: 'jsdom' } });
export default mergeConfig(base, overrides);`,
    `export default defineConfig(mergeConfig(
      { test: { clearMocks: true } },
      defineProject({ test: { environment: 'jsdom' } }),
    ));`,
  ])('does not insert defaults into merged config fragments: %s', (body) => {
    const input = `import { defineConfig, defineProject, mergeConfig } from 'vitest/config';\n${body}`;
    const result = config(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'merged-config-defaults', severity: 'review' }),
    );
  });

  it('leaves defaults in imported merge fragments for review across migration runs', () => {
    const base = `import { defineConfig } from 'vitest/config';
export default defineConfig({ test: { clearMocks: true, browser: { locators: { exact: true } } } });`;
    const overrides = `import { defineConfig } from 'vitest/config';
export default defineConfig({ test: { browser: { enabled: true } } });`;
    const root = project({
      'vitest.config.ts': `import { mergeConfig } from 'vitest/config';
import base from './base';
import overrides from './overrides';
export default mergeConfig(base, overrides);`,
      'base.ts': base,
      'overrides.ts': overrides,
    });
    const plan = planProject(root);
    expect(plan.changes).toEqual([]);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        file: path.join(root, 'overrides.ts'),
        code: 'merged-config-defaults',
      }),
    );
    applyVitestV5Migration(plan);
    const findings = finishVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'base.ts'), 'utf8')).toBe(base);
    expect(fs.readFileSync(path.join(root, 'overrides.ts'), 'utf8')).toBe(overrides);
    expect(findings.some(({ code }) => code === 'merged-config-defaults')).toBe(true);
    expect(planProject(root).changes).toEqual([]);
  });

  it.each(['true', 'false'])(
    'preserves inherited locators.exact: %s without a child override',
    (exact) => {
      const result = config(`export default { test: {
  browser: { enabled: true, locators: { exact: ${exact} } },
  projects: [{ extends: true, test: { browser: { enabled: true } } }],
} };`);
      expect(result.content.match(/locators:/g)).toHaveLength(1);
      expect(result.content).toContain(`locators: { exact: ${exact} }`);
      expect(result.findings).toEqual([]);
      expect(config(result.content).content).toBe(result.content);
    },
  );

  it('adds locator defaults to an inheriting child when its parent has no browser options', () => {
    const result = config(`export default { test: {
  projects: [{ extends: true, test: { browser: { enabled: true } } }],
} };`);
    expect(result.content).toContain('locators: { exact: false }');
    expect(result.findings).toEqual([]);
  });

  it('adds the locator default only to the parent browser config', () => {
    const result = config(`export default { test: {
  browser: { enabled: true },
  projects: [{ extends: true, test: { browser: { enabled: true } } }],
} };`);
    expect(result.content.match(/locators:/g)).toHaveLength(1);
    expect(result.content).toContain('locators: { exact: false }');
  });

  it('does not override dynamic inherited browser settings', () => {
    const result = config(`export default { test: {
  browser: sharedBrowser,
  projects: [{ extends: true, test: { browser: { enabled: true } } }],
} };`);
    expect(result.content).not.toContain('exact: false');
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'dynamic-project', severity: 'review' }),
    );
  });

  it('does not guess dynamic projects or spread options', () => {
    const result = config(`export default { test: { ...shared, projects: [() => project] } };`);
    expect(result.content).not.toContain('clearMocks');
    expect(result.findings.map((finding) => finding.code)).toContain('dynamic-config');
    const dynamic = config(
      `export default { test: { projects: [() => project, Promise.resolve(project), project] } };`,
    );
    expect(dynamic.findings.filter((finding) => finding.code === 'dynamic-project')).toHaveLength(
      3,
    );
  });

  it('moves browser.api without removing adjacent comments or properties', () => {
    for (const props of [
      'api: { port: 4444 }, /* keep, this */ enabled: true',
      'enabled: true, /* keep, this */ api: { port: 4444 }',
      '/* keep, this */ api: { port: 4444 },',
    ]) {
      const result = config(`export default { test: { browser: { ${props} } } };`);
      expect(result.content).toContain('api: { port: 4444 }');
      expect(result.content.match(/api:/g)).toHaveLength(1);
      expect(result.content).toContain('/* keep, this */');
      expect(result.findings).toEqual([]);
    }
  });

  it('blocks conflicting API settings', () => {
    const result = config(
      `export default { test: { api: { port: 1 }, browser: { api: { port: 2 } } } };`,
    );
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'api-conflict', severity: 'block' }),
    );
  });

  it.each([
    ['true', 'true'],
    ['false', 'false'],
    ['51204', '0xc804'],
    [`{ host: 'localhost', port: 51204 }`, `{ "port": 51204, 'host': "localhost" }`],
    [
      `{ port: 51204, allowWrite: false, allowExec: false }`,
      `{ allowExec: false, allowWrite: false, port: 51204 }`,
    ],
  ])('deduplicates equivalent static API values: %s and %s', (api, browserApi) => {
    const result = config(
      `export default { test: { api: ${api}, browser: { api: ${browserApi}, enabled: false } } };`,
    );
    expect(result.findings).toEqual([]);
    expect(result.content.match(/\bapi:/g)).toHaveLength(1);
    expect(result.content).toContain(`api: ${api}`);
    expect(config(result.content).content).toBe(result.content);
  });

  it.each([
    ['options', 'options'],
    ['getApi()', 'getApi()'],
    [`{ port: 1, ...options }`, `{ port: 1, ...options }`],
    [`{ get port() { return 1 } }`, `{ get port() { return 1 } }`],
    [`{ port: 1, port: 2 }`, `{ port: 1, port: 2 }`],
    [`{ port: 1 }`, `{ port: 1, host: '0.0.0.0' }`],
    [`{ allowExec: false }`, `{ allowExec: true }`],
  ])(
    'does not choose between unresolved or conflicting API values: %s and %s',
    (api, browserApi) => {
      const result = config(
        `export default { test: { api: ${api}, browser: { api: ${browserApi} } } };`,
      );
      expect(result.findings).toContainEqual(
        expect.objectContaining({ code: 'api-conflict', severity: 'block' }),
      );
      expect(result.content.match(/\bapi:/g)).toHaveLength(2);
    },
  );

  it('copies screenshots, glob perFile, and stdout defaults', () => {
    const result = config(`export default { test: {
browser: { screenshotDirectory: 'screens', expect: { toMatchScreenshot: { threshold: 0.1 } } },
coverage: { include: ['src'], thresholds: { perFile: true, 'src/**': { lines: 90 }, 'lib/**': { perFile: false } } },
reporters: ['default', 'json', ['junit', {}], ['html', { outputFile: 'reports/index.html' }]],
} };`);
    expect(result.content.match(/screenshotDirectory: 'screens'/g)).toHaveLength(2);
    expect(result.content.match(/perFile: true/g)).toHaveLength(2);
    expect(result.content).toContain('perFile: false');
    expect(result.content.match(/stdout: true/g)).toHaveLength(2);
    expect(result.content).toContain('outputDir: "reports"');
    expect(result.findings.map((finding) => finding.code)).toEqual(['coverage-patterns']);
    expect(config(result.content).content).toBe(result.content);
  });

  it('keeps explicit report destinations and reviews non-index HTML files', () => {
    const result = config(
      `export default { test: { outputFile: { json: 'report.json' }, reporters: ['json', ['junit', { stdout: false }], ['html', { outputFile: 'custom.html' }]] } };`,
    );
    expect(result.content).not.toContain('stdout: true');
    expect(result.content).toContain('stdout: false');
    expect(result.findings.map((finding) => finding.code)).toContain('html-output');
  });

  it.each([true, false])('retains supported benchmark options (preserveV4=%s)', (preserveV4) => {
    const input = `export default { test: { clearMocks: false, benchmark: {
      enabled: true, include: ['**/*.bench.ts'], exclude: [], retainSamples: true,
    } } };`;
    expect(config(input, preserveV4)).toEqual({ content: input, findings: [] });
  });

  it.each(['reporters', 'compare'])('blocks the removed benchmark.%s option', (key) => {
    const result = config(`export default { test: { benchmark: { ${key}: 'old' } } };`);
    expect(result.findings).toEqual([
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    ]);
    expect(result.findings[0].message).toContain(`benchmark.${key}`);
  });

  it('reports dynamic benchmark options for review', () => {
    expect(config('export default { test: { benchmark: options } };').findings).toEqual([
      expect.objectContaining({ code: 'benchmark-api', severity: 'review' }),
    ]);
  });

  it('preserves an installed Temporal polyfill only when toNotFake was omitted', () => {
    const result = migrateVitestV5Config('vite.config.ts', 'export default { test: {} };', {
      preserveV4: true,
      temporalPolyfill: true,
    });
    expect(result.content).toContain("toNotFake: ['Temporal']");
    const explicit = migrateVitestV5Config(
      'vite.config.ts',
      'export default { test: { fakeTimers: { toNotFake: [] } } };',
      { preserveV4: true, temporalPolyfill: true },
    );
    expect(explicit.content).toContain('toNotFake: []');
  });
});

describe('Vitest v5 source migration', () => {
  it('initializes runner aliases before executable code and retains directives', () => {
    const result = source(
      `'use strict';\nconst fn = getFn({});\nimport { getFn } from '@vitest/runner';`,
    );
    expect(result.findings).toEqual([]);
    expect(result.content.startsWith("'use strict';")).toBe(true);
    expect(result.content.indexOf('const getFn =')).toBeLessThan(
      result.content.indexOf('const fn ='),
    );
    const executable = result.content.replace(/import \{[^}]+\} from 'vite-plus\/test';/, '');
    expect(
      runInNewContext(`${executable}\nfn;`, { _VitestTestRunner: { getTestFn: () => 42 } }),
    ).toBe(42);
    expect(source(result.content).content).toBe(result.content);
  });

  it.each([
    `import runner, { getFn } from '@vitest/runner';`,
    `import runner from '@vitest/runner';`,
    `import * as runner from '@vitest/runner';`,
  ])('retains blocked default and namespace import syntax: %s', (input) => {
    const result = source(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'removed-api', severity: 'block' }),
    );
  });

  it('migrates a sequential modifier and its redundant option together', () => {
    const result = source(
      `import { test } from 'vitest'; test.sequential('x', { sequential: true }, () => {});`,
    );
    expect(result.content).toBe(
      `import { test } from 'vitest'; test('x', { concurrent: false }, () => {});`,
    );
    expect(result.findings).toEqual([]);
    expect(source(result.content).content).toBe(result.content);
  });

  it.each(['test', 'it'])('preserves the numeric timeout of %s.sequential', (api) => {
    for (const callback of ['async () => {}', 'function () {}']) {
      const input = `import { ${api} } from 'vitest'; ${api}.sequential('slow', ${callback}, 15_000);`;
      const result = source(input);
      expect(result.content).toBe(
        `import { ${api} } from 'vitest'; ${api}('slow', { concurrent: false, timeout: 15_000 }, ${callback});`,
      );
      expect(result.findings).toEqual([]);
      expect(source(result.content).content).toBe(result.content);
      const calls: unknown[][] = [];
      runInNewContext(result.content.replace(/^import[^;]+;/, ''), {
        [api]: (...args: unknown[]) => calls.push(args),
      });
      expect(calls[0]).toHaveLength(3);
      expect(calls[0][1]).toEqual({ concurrent: false, timeout: 15000 });
    }
  });

  it.each([
    `test.sequential('slow', async () => {}, timeout);`,
    `it.sequential('slow', async () => {}, { timeout: 15000 });`,
    `test.sequential('slow', async () => {}, ...options);`,
    `it.sequential('slow', async () => {}, 15000, extra);`,
    `test.sequential('slow', async () => {}, /* keep timeout comment */ 15000);`,
  ])('preserves sequential calls with unresolved trailing arguments: %s', (call) => {
    const input = `import { test, it } from 'vitest'; ${call}`;
    const result = source(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(expect.objectContaining({ code: 'sequential-api' }));
  });

  it.each([
    `test.sequential('x', { sequential: false }, () => {});`,
    `test.sequential('x', { sequential: true, concurrent: true }, () => {});`,
    `test.sequential.each([1])('x', { sequential: true }, () => {});`,
  ])('leaves unresolved sequential calls intact: %s', (call) => {
    const input = `import { test } from 'vitest'; ${call}`;
    const result = source(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(expect.objectContaining({ code: 'sequential-api' }));
  });

  it('reports actual DOM globals but ignores locally shadowed identifiers and objects', () => {
    const result = source(
      `function f(navigator, window, globalThis, global) { navigator = {}; window.navigator = {}; globalThis.navigator = {}; global.navigator = {}; }\nnavigator = {}; window.navigator = {};`,
    );
    expect(result.findings.filter(({ code }) => code === 'dom-global')).toHaveLength(2);
    expect(result.findings.every(({ line }) => line === 2)).toBe(true);
  });
  it.each(['./runners', '../runners', 'custom/runners'])(
    'leaves unrelated %s modules unchanged',
    (specifier) => {
      const input = `export { helper } from '${specifier}';
export * from '${specifier}';
const dynamic = await import('${specifier}');
const commonjs = require('${specifier}');
type Runner = import('${specifier}').Runner;`;
      expect(source(input)).toEqual({ content: input, findings: [] });
    },
  );

  it.each(['vitest/runners', 'vite-plus/test/runners'])(
    'still reports removed %s re-exports and indirect imports',
    (specifier) => {
      const result = source(`export { VitestTestRunner } from '${specifier}';
export * from '${specifier}';
const dynamic = await import('${specifier}');
const commonjs = require('${specifier}');
type Runner = import('${specifier}').VitestTestRunner;`);
      expect(result.findings.filter(({ severity }) => severity === 'block')).toHaveLength(4);
      expect(result.findings.filter(({ severity }) => severity === 'review')).toHaveLength(1);
      expect(result.findings.every(({ code }) => code === 'removed-api')).toBe(true);
    },
  );

  it('preserves computed matcher keys and remains idempotent', () => {
    const result = source(`import { test, expect } from 'vitest';
test('text', async () => {
  await expect.element(el)['toHaveTextContent']('partial');
  await expect.element(el)["toHaveTextContent"](/partial/);
});`);
    expect(result.content).toContain('["toMatchTextContent"](\'partial\')');
    expect(result.content).toContain('["toMatchTextContent"](/partial/)');
    expect(result.findings).toEqual([]);
    expect(source(result.content).content).toBe(result.content);
  });

  it('does not report matcher declarations that already use both v5 parameters', () => {
    const input = `declare module 'vite-plus/test' {
  interface Matchers<R extends void | Promise<void> = void | Promise<void>, T = unknown> {
    toEqualReceived(value: T): R;
  }
}`;
    expect(source(input, { preserveV4: false, browser: false })).toEqual({
      content: input,
      findings: [],
    });
    expect(source('interface Assertion<T> {}').findings).toEqual([
      expect.objectContaining({ code: 'assertion-types', severity: 'review' }),
    ]);
  });

  it('handles namespace imports without touching a shadowed namespace', () => {
    const result = source(`import * as v from 'vitest';
v.test.sequential('works', () => { v.expect(Promise.resolve(1)).resolves.toBe(1); });
v.test('mock', () => v.vi.mock('./a'));
function helper(v) { v.test.sequential('local', () => {}); }`);
    expect(result.content).toContain("v.test('works', { concurrent: false }, async () =>");
    expect(result.content).toContain('await v.expect(Promise.resolve(1)).resolves.toBe(1)');
    expect(result.content).toContain("v.test.sequential('local', () => {})");
    expect(result.findings.map((finding) => finding.code)).toContain('nested-hoisted-mock');
  });

  it('reports type imports without blocking runtime migration', () => {
    const result = source(`type Runner = import('vitest/internal/module-runner').ModuleRunner;`);
    expect(result.findings).toEqual([
      expect.objectContaining({ severity: 'review', code: 'removed-api' }),
    ]);
  });

  it('does not add constructor warnings for ordinary function mocks', () => {
    expect(
      source(`import { vi } from 'vitest'; const fn = vi.fn(() => 1); vi.spyOn(console, 'log');`)
        .findings,
    ).toEqual([]);
  });
  it('rewrites safe sequential calls and assertions together', () => {
    const result = source(`import { test as check, expect } from 'vitest';
check.sequential('works', () => { expect(Promise.resolve(1)).resolves.toBe(1); });
check('options', { sequential: true }, () => {});
expect(() => {}).toThrow('');
expect.element(el).toHaveTextContent('partial');
expect.element(el).toHaveTextContent(/partial/);`);
    expect(result.content).toContain("check('works', { concurrent: false }, async () =>");
    expect(result.content).toContain('await expect(Promise.resolve(1)).resolves.toBe(1)');
    expect(result.content).toContain('toThrow(/^$/)');
    expect(result.content.match(/toMatchTextContent/g)).toHaveLength(2);
    expect(source(result.content).content).toBe(result.content);
  });

  it('respects local aliases and shadowed bindings', () => {
    const input = `import { test, expect as assert } from 'vitest';
function helper(test, assert) { test.sequential('x', () => {}); assert(() => {}).toThrow(''); }
test('x', () => assert(() => {}).toThrow(''));`;
    const result = source(input);
    expect(result.content).toContain("test.sequential('x', () => {})");
    expect(result.content).toContain("assert(() => {}).toThrow('')");
    expect(result.content).toContain('toThrow(/^$/)');
  });

  it.each(['@vitest/expect', 'vite-plus/test/plugins/expect'])(
    'migrates assertions from %s in the same pass as their imports',
    (specifier) => {
      const result = source(`import { expect as check } from '${specifier}';
import { test } from 'vitest';
function helper(check) { check(() => {}).toThrow(''); }
test('works', () => {
  check(() => { throw new Error('boom'); }).not.toThrow('');
  check(Promise.resolve(1)).resolves.toBe(1);
  check.element(el).toHaveTextContent('partial');
});`);
      expect(result.content).toContain("import { expect as check } from 'vite-plus/test'");
      expect(result.content).toContain("function helper(check) { check(() => {}).toThrow(''); }");
      expect(result.content).toContain('.not.toThrow(/^$/)');
      expect(result.content).toContain("test('works', async () =>");
      expect(result.content).toContain('await check(Promise.resolve(1)).resolves.toBe(1)');
      expect(result.content).toContain("await check.element(el).toMatchTextContent('partial')");
      expect(result.findings).toEqual([]);
      expect(source(result.content).content).toBe(result.content);
    },
  );

  it('only makes async-compatible callbacks async', () => {
    const result = source(`import { test, describe, expect } from 'vitest';
import { render as mount } from 'vitest-browser-vue';
test('component', () => { const screen = mount(Component); expect(Promise.resolve(1)).resolves.toBe(1); });
describe('suite', () => { mount(Component); });
function publicSync() { return { screen: mount(Component) }; }`);
    expect(result.content).toContain("test('component', async () =>");
    expect(result.content).toContain('(await mount(Component))');
    expect(result.content).toContain("describe('suite', () => { mount(Component); })");
    expect(result.findings.filter((finding) => finding.code === 'async-render')).toHaveLength(2);
  });

  it('migrates runner aliases and preserves unsupported type-only imports for review', () => {
    const result =
      source(`import { test as check, getFn as lookup, type File as MyFile, type SuiteHooks } from '@vitest/runner';
import { createExpect as makeExpect } from '@vitest/expect';
lookup(task);`);
    expect(result.content).toContain('test as check');
    expect(result.content).toContain('type RunnerTestFile as MyFile');
    expect(result.content).toContain('const lookup = _VitestTestRunner.getTestFn');
    expect(result.content).toContain('createExpect as makeExpect');
    expect(result.findings).toContainEqual(
      expect.objectContaining({ severity: 'review', code: 'removed-api' }),
    );
    expect(result.findings.some((finding) => finding.severity === 'block')).toBe(false);
  });

  it('blocks unsupported active internals while migrating direct benchmarks', () => {
    const result = source(`import { startTests } from '@vitest/runner';
import * as internals from 'vitest/internal/module-runner';
import { bench } from 'vitest';
bench('old', () => {});`);
    expect(result.findings.filter((finding) => finding.severity === 'block')).toHaveLength(2);
  });

  it('migrates global benchmarks but preserves the new test-context fixture', () => {
    const result = migrateVitestV5Source(
      'example.test.ts',
      `bench('old', () => {});
test('new', ({ bench }) => { bench('new', () => {}); });`,
      { ...v4, globals: true },
    );
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`globalThis.test('old'`);
    expect(result.content).toContain(`test('new', ({ bench }) => { bench('new', () => {}); });`);
  });

  it('rewrites resolveConfig destructuring with aliases and collect options', () => {
    const result = source(`import { resolveConfig as resolve, createVitest } from 'vitest/node';
const { viteConfig: vite, vitestConfig: test } = await resolve();
const runner = await createVitest('test', {});
await runner.collect();
await runner.collect(['unit'], {});
await runner.collect([], { staticParse: true });
other.collect(options);`);
    expect(result.content).toContain('const vite = await resolve(), test = vite.test');
    expect(result.content).toContain('runner.collect(undefined, { staticParse: false })');
    expect(result.content).toContain("runner.collect(['unit'], { staticParse: false })");
    expect(result.content).toContain('staticParse: true');
    expect(result.findings.map((finding) => finding.code)).toEqual(['static-collect']);
    expect(source(result.content).content).toBe(result.content);
  });

  it('reports manual migration risks with line locations', () => {
    const result = source(`import { vi, test, expect } from 'vitest';
import '@vitest/ws-client';
test('mock', () => { vi.mock('./a'); vi.fn(class {}); });
process.env.VITEST_POOL_ID;
globalThis.navigator = value;
globalThis.foo = originals.get('foo');
Temporal.Now.instant(); vi.setSystemTime(0);
const ui = 'http://localhost:51204/__vitest__/';
const browser = 'http://localhost:63315/__vitest_test__/';
interface Assertion<T> {}
expect.poll(() => 1).toBe(1);`);
    expect(result.findings.map((finding) => finding.code)).toEqual(
      expect.arrayContaining([
        'ws-client',
        'nested-hoisted-mock',
        'browser-automock',
        'class-mock',
        'worker-id',
        'dom-global',
        'global-descriptors',
        'temporal-system-time',
        'ui-token',
        'browser-session',
        'assertion-types',
        'poll-timeout',
        'unawaited-assertion',
      ]),
    );
    expect(result.findings.every((finding) => finding.line > 1)).toBe(true);
  });
});

describe('Vitest v5 command migration', () => {
  it.each([
    ['vitest --reporter=json | jq', 'reporter-stdout', 'review'],
    ['vitest run -t "suite test"', 'test-name-pattern', 'review'],
    ['vitest --compare=baseline.json', 'benchmark-api', 'block'],
    ['vitest --outputJson=$BASELINE', 'benchmark-api', 'block'],
    ['vitest && cp -r .vitest-attachements artifacts', 'artifact-paths', 'review'],
  ])('reports %s without changing it', (command, code, severity) => {
    const result = migrateVitestV5Command('package.json', command, true, 7);
    expect(result.content).toBe(command);
    expect(result.findings).toContainEqual(expect.objectContaining({ code, severity, line: 7 }));
  });

  it('does not preserve v4 collection defaults for a v5 project', () => {
    expect(migrateVitestV5Command('package.json', 'vitest list', false)).toEqual({
      content: 'vitest list',
      findings: [],
    });
  });
  it.each([
    'vitest list',
    'pnpm exec vitest list',
    'npm exec -- vitest list',
    'npx vitest list',
    'vp test list',
  ])('preserves runtime collection for %s', (command) => {
    const result = migrateVitestV5Command('package.json', `${command} 'unit tests'`, true);
    expect(result.content).toBe(`${command} --no-static-parse 'unit tests'`);
    expect(migrateVitestV5Command('package.json', result.content, true).content).toBe(
      result.content,
    );
  });
  it.each([
    'vitest list --static-parse',
    'vitest list --no-static-parse',
    'vitest list --static-parse=false',
  ])('keeps explicit flags: %s', (command) => {
    expect(migrateVitestV5Command('package.json', command, true).content).toBe(command);
  });
  it.each([
    'vitest list | jq .',
    'NODE_ENV=test vitest list',
    'vitest list "$FILTER"',
    'wrapper vitest list',
    'vitest list && echo done',
  ])('reports uncertain commands: %s', (command) => {
    const result = migrateVitestV5Command('ci.yml', command, true);
    expect(result.content).toBe(command);
    expect(result.findings).toContainEqual(expect.objectContaining({ code: 'static-list' }));
  });
  it('does not interpret filter arguments after -- as options', () => {
    expect(
      migrateVitestV5Command('package.json', 'vitest list -- --static-parse', true).content,
    ).toBe('vitest list --no-static-parse -- --static-parse');
  });
});

describe('Vitest v5 versioned preflight', () => {
  it.each(['catalog:', 'catalog:testing'])(
    'resolves the installed runner of a linked Vite+ build with %s',
    (specifier) => {
      const root = project({
        'package.json': JSON.stringify({ devDependencies: { 'vite-plus': 'latest' } }),
        'vite.config.ts': 'export default { test: {} };',
        'node_modules/vite-plus/package.json': JSON.stringify({
          name: 'vite-plus',
          version: '0.0.0',
          dependencies: { vitest: specifier },
        }),
        'node_modules/vite-plus/node_modules/vitest/package.json': JSON.stringify({
          name: 'vitest',
          version: '5.0.1',
        }),
        // The project catalog does not describe the linked toolchain's runner.
        'pnpm-workspace.yaml': 'catalog:\n  vitest: 4.1.11\n',
      });
      const plan = planProject(root);
      expect(plan.projects[0].sourceVersion).toBe('5.0.1');
      expect(plan.changes).toEqual([]);
      expect(plan.findings).toEqual([]);
      expect(vitestV5NeedsMigration(plan)).toBe(false);
      expect(fs.existsSync(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
    },
  );

  it('retains original activity after bootstrap removes a redundant direct runner', () => {
    const root = project();
    const original = planProject(root);
    fs.writeFileSync(path.join(root, 'package.json'), '{}');
    const refreshed = refreshVitestV5Migration(original);
    expect(refreshed.projects[0]).toMatchObject({
      active: true,
      sourceVersion: '4.1.0',
      options: { preserveV4: true },
    });
    expect(refreshed.findings).toContainEqual(
      expect.objectContaining({ code: 'configless-defaults' }),
    );
    applyVitestV5Migration(refreshed);
    expect(finishVitestV5Migration(refreshed)).toContainEqual(
      expect.objectContaining({ code: 'configless-defaults' }),
    );
    expect(planProject(root).projects[0].sourceVersion).toBeUndefined();
  });

  it.each(['4.1.11', '5.0.0'])(
    'does not infer original test usage from a newly added peer runner %s',
    (version) => {
      const root = project({ 'package.json': '{}' });
      const original = planProject(root);
      expect(original.projects[0].active).toBe(false);
      fs.writeFileSync(
        path.join(root, 'package.json'),
        JSON.stringify({ devDependencies: { vitest: version } }),
      );
      const refreshed = refreshVitestV5Migration(original);
      expect(refreshed.projects[0]).toMatchObject({
        active: false,
        sourceVersion: undefined,
        options: { preserveV4: false },
      });
      expect(refreshed.findings).toEqual([]);
      expect(refreshed.changes).toEqual([]);
    },
  );
  it.each(['', '\uFEFF'])(
    'preserves manifest formatting and BOM %j when changing a script',
    (bom) => {
      const before = `${bom}{\r\n\t"name": "test",\r\n\t"scripts": { "test": "vitest list", "filter": "vitest -t suite" },\r\n\t"devDependencies": {"vitest":"^4.1.0"}\r\n}`;
      const root = project({ 'package.json': before });
      const plan = planProject(root);
      expect(plan.changes.find(({ file }) => file === path.join(root, 'package.json'))?.after).toBe(
        before.replace('vitest list', 'vitest list --no-static-parse'),
      );
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'test-name-pattern', line: 3 }),
      );
      applyVitestV5Migration(plan);
      expect(planProject(root).changes).toEqual([]);
    },
  );

  it.each(['vitest list', ['vitest list'], null])(
    'ignores malformed scripts containers: %j',
    (scripts) => {
      const root = project({
        'package.json': JSON.stringify({ scripts, devDependencies: { vitest: '^4.1.0' } }),
      });
      expect(planProject(root).changes).toEqual([]);
    },
  );

  it.each(['4.1.11', '5.0.0'])(
    'refreshes changed setup inputs without losing original Vitest %s',
    (version) => {
      const before = JSON.stringify({
        devDependencies: { vitest: version },
        scripts: { test: 'vitest list' },
      });
      const root = project({
        'package.json': before,
        'vite.config.ts': 'export default { test: {} };',
      });
      const original = planProject(root);
      expect(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).toBe(before);
      // Model an earlier install/tool migration changing a manifest and config.
      fs.writeFileSync(
        path.join(root, 'package.json'),
        JSON.stringify({
          devDependencies: { vitest: '5.0.0' },
          scripts: { test: 'vitest list', lint: 'oxlint' },
        }),
      );
      fs.writeFileSync(
        path.join(root, 'vite.config.ts'),
        'export default { test: { name: "keep" } };',
      );
      const refreshed = refreshVitestV5Migration(original);
      expect(refreshed.projects[0].sourceVersion).toBe(version);
      applyVitestV5Migration(refreshed);
      const manifest = fs.readFileSync(path.join(root, 'package.json'), 'utf8');
      expect(manifest).toContain('oxlint');
      expect(manifest.includes('--no-static-parse')).toBe(version.startsWith('4'));
      expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toContain('name: "keep"');
      expect(fs.existsSync(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
      fs.writeFileSync(path.join(root, 'vite.config.ts'), 'export default {};');
      expect(() => applyVitestV5Migration(refreshed)).toThrow('Migration input changed');
    },
  );
  it.each([
    ['an unpaired surrogate', String.raw`'\ud800'`],
    ['a deep expression', Array(160).fill('1').join(' + ')],
  ])('migrates removed APIs in a file with %s without blocking preflight', (_name, expression) => {
    const declaration = `const value = ${expression};`;
    const root = project({
      'example.test.ts': `import { getFn } from '@vitest/runner';\n${declaration}`,
    });
    const plan = planProject(root);
    expect(plan.findings.some(({ code }) => code === 'source-parse')).toBe(false);
    expect(plan.findings.some(({ severity }) => severity === 'block')).toBe(false);
    const change = plan.changes.find(({ file }) => file === path.join(root, 'example.test.ts'));
    expect(change?.after).toContain('const getFn = _VitestTestRunner.getTestFn;');
    expect(change?.after).toContain(declaration);
  });

  it('keeps Flow files unchanged and reports unsupported syntax before migration', () => {
    const input = `// @flow\nimport { expect } from 'vitest';\nconst value: string = '';\nexpect(() => {}).toThrow('');`;
    const root = project({ 'flow.test.js': input });
    const file = path.join(root, 'flow.test.js');
    const plan = planProject(root);
    expect(plan.changes.some((change) => change.file === file)).toBe(false);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        file,
        code: 'source-parse',
        message: expect.stringContaining('Flow is not supported'),
      }),
    );
    expect(fs.readFileSync(file, 'utf8')).toBe(input);
  });

  it.each([
    `import { defineConfig } from 'vitest/config';
export default defineConfig(CONFIG);`,
    `const config = CONFIG; export default config;`,
    `module.exports = CONFIG;`,
  ])('does not treat plugin projects or extends as Vitest config references: %s', (wrapper) => {
    const root = project({
      'vite.config.ts': wrapper.replace(
        'CONFIG',
        `{
        plugins: [paths({ projects: ['./tsconfig.json', './plugin-options.ts'], extends: './plugin-base.ts' })],
        test: { projects: [{ extends: './test-base.ts', test: { projects: ['./checks.ts'] } }] },
      }`,
      ),
      'tsconfig.json': '{ "compilerOptions": {} }',
      'plugin-options.ts': 'export default { test: {} };',
      'plugin-base.ts': 'export default { test: {} };',
      'test-base.ts': 'export default { test: {} };',
      'checks.ts': 'export default { test: {} };',
    });
    const plan = planProject(root);
    expect([...plan.projects[0].configFiles].map((file) => path.basename(file)).toSorted()).toEqual(
      ['checks.ts', 'test-base.ts', 'vite.config.ts'],
    );
    expect(plan.findings.some(({ code }) => code === 'source-parse')).toBe(false);
    expect(plan.changes.some(({ file }) => /plugin-(?:options|base)\.ts$/.test(file))).toBe(false);
  });

  it('reports the Jest-only DOM type entry without guessing mixed test configuration', () => {
    const root = project({
      'vite.config.ts': 'export default { test: {} };',
      'tsconfig.json':
        '{ // inherited setup files need review\n"compilerOptions": { "types": ["@testing-library/jest-dom", "vitest/globals"] } }',
    });
    const plan = planProject(root);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'jest-dom-types', severity: 'review' }),
    );
    expect(plan.changes.some(({ file }) => file === path.join(root, 'tsconfig.json'))).toBe(false);
    fs.writeFileSync(
      path.join(root, 'tsconfig.json'),
      JSON.stringify({
        compilerOptions: {
          types: [
            '@testing-library/jest-dom',
            '@testing-library/jest-dom/vitest',
            'vitest/globals',
          ],
        },
      }),
    );
    expect(planProject(root).findings.some(({ code }) => code === 'jest-dom-types')).toBe(false);
  });

  it('does not mistake ambient config declarations or type imports for a config', () => {
    const root = project({
      'types.ts':
        "import type { UserConfig } from 'vitest/config';\ndeclare module 'vitest/config' { const config: UserConfig; export default config; }",
    });
    const plan = planProject(root);
    expect(plan.projects[0].configFiles.size).toBe(0);
    expect(plan.findings.map((finding) => finding.code)).toEqual(['configless-defaults']);
  });

  it('retains the Yarn catalog resolver when scanning after dependency updates', () => {
    const root = project({
      'package.json': JSON.stringify({ devDependencies: { '@vitest/ui': '^4.1.11' } }),
      '.yarnrc.yml': 'catalog:\n  vitest: 5.0.0\n',
    });
    const plan = planVitestV5Migration({ rootDir: root, packageManager: PackageManager.yarn });
    expect(plan.projects[0].active).toBe(false);
    applyVitestV5Migration(plan);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: 'catalog:' } }),
    );
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });
  it('migrates raw referenced configs with custom filenames and nested references', () => {
    const root = project({
      'vitest.config.ts': "export default { test: { projects: ['./node-tests.ts'] } };",
      'node-tests.ts': "export default { test: { projects: ['./browser-tests.ts'] } };",
      'browser-tests.ts': 'export default { test: { browser: { enabled: true } } };',
    });
    const plan = planProject(root);
    expect(plan.projects[0].configFiles.size).toBe(3);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'node-tests.ts'), 'utf8')).toContain(
      'clearMocks: false',
    );
    expect(fs.readFileSync(path.join(root, 'browser-tests.ts'), 'utf8')).toContain('exact: false');
  });

  it('follows project globs across workspace package boundaries', () => {
    const root = project({
      'vitest.config.ts': "export default { test: { projects: ['./packages/*/checks.ts'] } };",
      'packages/unit/package.json': '{}',
      'packages/unit/checks.ts': 'export default { test: {} };',
    });
    const plan = planVitestV5Migration({
      rootDir: root,
      packageManager: PackageManager.pnpm,
      packages: [{ name: 'unit', path: 'packages/unit' }],
    });
    expect(plan.projects[1].configFiles.size).toBe(1);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'packages/unit/checks.ts'), 'utf8')).toContain(
      'clearMocks: false',
    );
    expect(vitestV5ConfiglessProjects(plan)).not.toContain(path.join(root, 'packages/unit'));
    expect(fs.existsSync(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
  });

  it('rejects runtime input changes between preflight and application', () => {
    const root = project({
      '.node-version': '22.18.0',
      'vite.config.ts': 'export default { test: {} };',
    });
    const plan = planProject(root);
    fs.writeFileSync(path.join(root, '.node-version'), '20.19.0');
    expect(() => applyVitestV5Migration(plan)).toThrow('Migration input changed');
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).not.toContain('clearMocks');
  });

  it('does not require a Vitest source version for a Vite+-only build project', () => {
    const root = project({
      'package.json': JSON.stringify({ devDependencies: { 'vite-plus': '0.2.0' } }),
      'vite.config.ts':
        "import { defineConfig } from 'vite-plus'; export default defineConfig({});",
    });
    expect(planProject(root).findings).toEqual([]);
    expect(planProject(root).projects[0].active).toBe(false);
  });

  it('does not mistake a parameter named test for Vitest usage', () => {
    const root = project({
      'package.json': '{}',
      'business.ts': 'export function run(test: string) { return test; }',
    });
    expect(planProject(root).findings).toEqual([]);
    expect(planProject(root).projects[0].active).toBe(false);
  });

  it('does not rewrite new v5 assertions in a migrated configless project without state', () => {
    const root = project();
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    fs.writeFileSync(
      path.join(root, 'new.test.ts'),
      "import { expect } from 'vite-plus/test'; expect(() => {}).toThrow('');",
    );
    expect(planProject(root).changes).toEqual([]);
  });
  it('does not treat stale overrides or transitive lockfile packages as active tests', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { 'vite-plus': 'latest' },
        overrides: { vitest: 'npm:@voidzero-dev/vite-plus-test@latest' },
      }),
      'pnpm-lock.yaml': "packages:\n  '@vitest/spy@4.1.11': {}\n",
    });
    expect(planProject(root).findings).toEqual([]);
    expect(planProject(root).projects[0].active).toBe(false);
  });

  it.each([
    "export const tools = ['vite', 'vitest', '@vitest/runner', 'vite-plus/test'];",
    "export const tool = { name: 'vitest' };",
    "// import { test } from 'vitest';\nexport const value = 1;",
    "/* export * from 'vitest'; vitest run */\nexport const value = 1;",
    'export const example = "import { test } from \'vitest\';";',
    'export const help = `Run vitest run or vp test`;',
    "console.log('vitest run');",
    "const execSync = (command) => command; execSync('vitest run');",
    "const { execSync } = require('./logger'); execSync('vitest run');",
    'const docs = `\n/// <reference types="vitest/globals" />\n`;',
    '/*\n/// <reference types="vitest/globals" />\n*/',
    "function require(name) { return name; } require('vitest');",
    "const loader = { require(name) { return name; } }; loader.require('vitest');",
    "import { expect } from 'vitest-like';",
    "export * from 'vite-plus/testing';",
  ])('does not require a runner version for incidental source text: %s', (input) => {
    const root = project({ 'package.json': '{}', 'src/plugin.ts': input });
    const plan = planProject(root);
    expect(plan.projects[0].active).toBe(false);
    expect(plan.findings).toEqual([]);
    expect(plan.changes).toEqual([]);
  });

  it.each([
    "import { test } from 'vitest';",
    "import 'vitest';",
    "import type { TestAPI } from 'vitest';",
    "export { test } from 'vite-plus/test';",
    "export * from '@vitest/runner';",
    "const runner = await import('vitest/node');",
    "const runner = require('vitest');",
    "import runner = require('vitest');",
    "type Runner = import('vitest/node').Vitest;",
    "declare module 'vitest' { interface ProvidedContext { port: number } }",
    '/// <reference types="vitest/globals" />',
    "import { execSync as run } from 'node:child_process'; run('pnpm exec vitest run');",
    "import * as cp from 'child_process'; cp.spawn('vitest', ['run']);",
    "import cp from 'node:child_process'; cp.execSync('vp test');",
    "import { spawnSync } from 'node:child_process'; spawnSync('pnpm', ['exec', 'vitest', 'run']);",
    "import { execFileSync } from 'child_process'; execFileSync('vp', ['test']);",
    "const { execSync: run } = require('node:child_process'); run('vitest run');",
    "const cp = require('child_process'); cp.spawn('vitest', ['run']);",
    "require('node:child_process').execSync('vp test');",
  ])('still requires a runner version for actual source usage: %s', (input) => {
    const root = project({ 'package.json': '{}', 'runner.ts': input });
    const plan = planProject(root);
    expect(plan.projects[0].active).toBe(true);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'source-version', severity: 'block' }),
    );
    expect(plan.changes).toEqual([]);
  });

  it('keeps the version check when source usage cannot be parsed', () => {
    const root = project({
      'package.json': '{}',
      'runner.ts': "import { test } from 'vitest'; const broken = ;",
    });
    const plan = planProject(root);
    expect(plan.projects[0].active).toBe(true);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'source-version', severity: 'block' }),
    );
  });

  it('migrates Vitest workspace members without activating their plugin package', () => {
    const plugin = "export const NON_RUNTIME_PKGS = ['vite', 'vitest'];";
    const root = project({
      'package.json': JSON.stringify({
        name: 'plugin',
        scripts: { test: 'node test-examples.js' },
      }),
      'src/index.ts': plugin,
      'examples/vite-8/package.json': JSON.stringify({ devDependencies: { vitest: '^4.1.11' } }),
      'examples/vite-8/vitest.config.ts': 'export default { test: { globals: true } };',
      'examples/vite-8/unit.test.js': "test.sequential('works', () => {});",
    });
    const workspace = {
      rootDir: root,
      packageManager: PackageManager.pnpm,
      packages: [{ name: 'example', path: 'examples/vite-8' }],
    };
    const plan = planVitestV5Migration(workspace);
    expect(plan.projects.map(({ active, sourceVersion }) => ({ active, sourceVersion }))).toEqual([
      { active: false, sourceVersion: undefined },
      { active: true, sourceVersion: '4.1.11' },
    ]);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toEqual([]);
    expect(fs.readFileSync(path.join(root, 'src/index.ts'), 'utf8')).toBe(plugin);
    expect(fs.readFileSync(path.join(root, 'examples/vite-8/unit.test.js'), 'utf8')).toContain(
      "test('works', { concurrent: false }",
    );
    const repeated = planVitestV5Migration(workspace);
    expect(repeated.projects[0].active).toBe(false);
    expect(repeated.findings).toEqual([]);
    expect(repeated.changes).toEqual([]);
    expect(fs.existsSync(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
  });

  it.each([
    ['vitest', '4.1.11', '4.1.11', {}],
    [
      'vitest',
      'npm:@voidzero-dev/vite-plus-test@latest',
      '@voidzero-dev/vite-plus-test@0.1.14',
      {},
    ],
    [
      'vite-plus',
      '^0.1.11',
      '0.1.14(peer@1.0.0)',
      {
        'vite-plus@0.1.14(peer@1.0.0)': {
          dependencies: { '@voidzero-dev/vite-plus-test': '0.1.14' },
        },
      },
    ],
    [
      'vite-plus',
      '^0.2.0',
      '0.2.0',
      {
        'vite-plus@0.2.0': { dependencies: { vitest: '4.1.11' } },
      },
    ],
  ])(
    'reads the original %s runner through pnpm lockfile edges (%s)',
    (name, spec, version, snapshots) => {
      const root = project({
        'package.json': JSON.stringify({
          scripts: { test: 'vp test' },
          devDependencies: { [name]: spec },
        }),
        // JSON is YAML: retain exact peer-suffixed snapshot keys in this fixture.
        'pnpm-lock.yaml': JSON.stringify({
          lockfileVersion: '9.0',
          importers: { '.': { devDependencies: { [name]: { specifier: spec, version } } } },
          packages: {
            '@voidzero-dev/vite-plus-test@0.1.14': { peerDependencies: { '@vitest/ui': '4.1.11' } },
            'vitest@5.0.0': {},
          },
          snapshots: { 'vitest@5.0.0': {}, ...snapshots },
        }),
      });
      expect(planProject(root).projects[0].sourceVersion).toBe('4.1.11');
      expect(planProject(root).findings.some(({ severity }) => severity === 'block')).toBe(false);
    },
  );

  it('does not infer a source runner from stale or unrelated lockfile entries', () => {
    const root = project({
      'package.json': JSON.stringify({
        scripts: { test: 'vp test' },
        devDependencies: { 'vite-plus': '^0.2.0' },
      }),
      'pnpm-lock.yaml': JSON.stringify({
        importers: {
          '.': { devDependencies: { 'vite-plus': { specifier: '^0.1.0', version: '0.1.0' } } },
        },
        snapshots: {
          'vite-plus@0.1.0': { dependencies: { vitest: '4.1.11' } },
          'vitest@5.0.0': {},
        },
      }),
    });
    expect(planProject(root).projects[0].sourceVersion).toBeUndefined();
    expect(planProject(root).findings).toContainEqual(
      expect.objectContaining({ code: 'source-version', severity: 'block' }),
    );
  });

  it.each(['^4.1.0 || ^5.0.0', '>=4', '*', '4 || 5.0.0-beta.1'])(
    'requires installed or locked evidence for the ambiguous runner range %s',
    (range) => {
      const root = project({
        'package.json': JSON.stringify({ devDependencies: { vitest: range } }),
        'vite.config.ts': 'export default { test: {} };',
      });
      const plan = planProject(root);
      expect(plan.projects[0].sourceVersion).toBeUndefined();
      expect(plan.projects[0].options.preserveV4).toBe(false);
      expect(plan.changes).toEqual([]);
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'source-version', severity: 'block' }),
      );
    },
  );

  it.each(['4.1.11', '5.0.0'])(
    'uses installed runner %s to disambiguate a cross-major range',
    (version) => {
      const root = project({
        'package.json': JSON.stringify({ devDependencies: { vitest: '^4.1.0 || ^5.0.0' } }),
        'node_modules/vitest/package.json': JSON.stringify({ name: 'vitest', version }),
      });
      const plan = planProject(root);
      expect(plan.projects[0].sourceVersion).toBe(version);
      expect(plan.projects[0].options.preserveV4).toBe(version.startsWith('4.'));
      expect(plan.findings.some(({ severity }) => severity === 'block')).toBe(false);
    },
  );

  it('uses the installed bundled runner before an ambiguous Vite+ dependency range', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { 'vite-plus': '0.2.0' },
        scripts: { test: 'vp test' },
      }),
      'node_modules/vite-plus/package.json': JSON.stringify({
        name: 'vite-plus',
        version: '0.2.0',
        dependencies: { vitest: '^4.1.0 || ^5.0.0' },
      }),
      'node_modules/vite-plus/node_modules/vitest/package.json': JSON.stringify({
        name: 'vitest',
        version: '5.0.0',
      }),
    });
    const plan = planProject(root);
    expect(plan.projects[0].sourceVersion).toBe('5.0.0');
    expect(plan.projects[0].options.preserveV4).toBe(false);
    expect(plan.findings.some(({ severity }) => severity === 'block')).toBe(false);
  });

  it.each(
    ['installed', 'locked'].flatMap((evidence) =>
      ['0.1.24', '3.2.4', '5.0.0'].map((version) => [evidence, version]),
    ),
  )(
    'ignores %s metadata for a legacy runner alias with an unrelated %s version',
    (evidence, version) => {
      const root = project({
        'package.json': JSON.stringify({ devDependencies: { vitest: 'catalog:' } }),
        'pnpm-workspace.yaml': JSON.stringify({
          catalog: { vitest: 'npm:@voidzero-dev/vite-plus-test@0.1.24' },
        }),
        'vite.config.ts': 'export default { test: {} };',
        ...(evidence === 'installed'
          ? {
              'node_modules/vitest/package.json': JSON.stringify({ name: 'vitest', version }),
            }
          : {
              'pnpm-lock.yaml': JSON.stringify({
                importers: {
                  '.': { devDependencies: { vitest: { specifier: 'catalog:', version } } },
                },
              }),
            }),
      });
      const plan = planProject(root);
      expect(plan.projects[0].sourceVersion).toBe('4.0.0');
      expect(plan.projects[0].options.preserveV4).toBe(true);
      expect(plan.findings.some(({ severity }) => severity === 'block')).toBe(false);
      expect(plan.changes.find(({ file }) => file.endsWith('vite.config.ts'))?.after).toContain(
        'clearMocks: false',
      );
    },
  );

  it('reads the upstream version from an installed legacy alias UI peer', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: 'npm:@voidzero-dev/vite-plus-test@0.1.24' },
      }),
      'node_modules/vitest/package.json': JSON.stringify({
        name: '@voidzero-dev/vite-plus-test',
        version: '0.1.24',
        peerDependencies: { '@vitest/ui': '4.1.11' },
      }),
    });
    expect(planProject(root).projects[0].sourceVersion).toBe('4.1.11');
  });

  it('reads the upstream version from an installed legacy wrapper UI peer', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { 'vite-plus': '0.1.14' },
        scripts: { test: 'vp test' },
      }),
      'node_modules/vite-plus/package.json': JSON.stringify({
        name: 'vite-plus',
        version: '0.1.14',
        dependencies: { '@voidzero-dev/vite-plus-test': '0.1.14' },
      }),
      'node_modules/@voidzero-dev/vite-plus-test/package.json': JSON.stringify({
        name: '@voidzero-dev/vite-plus-test',
        version: '0.1.14',
        peerDependencies: { '@vitest/ui': '4.1.11' },
      }),
    });
    expect(planProject(root).projects[0].sourceVersion).toBe('4.1.11');
  });
  it('uses an installed Vite+ runner version before an unrelated hoisted runner', () => {
    const root = project({
      'package.json': JSON.stringify({ devDependencies: { 'vite-plus': '0.2.0' } }),
      'node_modules/vite-plus/package.json': JSON.stringify({
        name: 'vite-plus',
        version: '0.2.0',
        dependencies: { vitest: '4.1.11' },
      }),
      'node_modules/vitest/package.json': JSON.stringify({ name: 'vitest', version: '5.0.0' }),
    });
    expect(planProject(root).projects[0].sourceVersion).toBe('4.1.11');
  });

  it('does not treat a retained v4 peer range as the installed Vite+ runner', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { 'vite-plus': '0.3.2' },
        peerDependencies: { vitest: '^4.0.0' },
        scripts: { test: 'vp test' },
      }),
      'node_modules/vite-plus/package.json': JSON.stringify({
        name: 'vite-plus',
        version: '0.3.2',
        dependencies: { vitest: '5.0.1' },
      }),
      'node_modules/vitest/package.json': JSON.stringify({ name: 'vitest', version: '4.1.11' }),
      'vite.config.ts': 'export default { test: {} };',
      'unit.test.js': "import { expect } from 'vite-plus/test'; expect(() => {}).toThrow('');",
    });
    const plan = planProject(root);
    expect(plan.projects[0].sourceVersion).toBe('5.0.1');
    expect(plan.findings).toEqual([]);
    expect(plan.changes).toEqual([]);
    expect(vitestV5NeedsMigration(plan)).toBe(false);
  });
  it('is read-only and blocks the entire plan before applying any safe edit', () => {
    const root = project({
      '.node-version': '20.19.0\n',
      'vite.config.ts': 'export default { test: {} };',
      'old.test.ts': "import { startTests } from '@vitest/runner'; startTests([]);",
    });
    const plan = planProject(root);
    expect(
      plan.findings
        .filter((finding) => finding.severity === 'block')
        .map((finding) => finding.code),
    ).toEqual(['removed-api']);
    expect(plan.changes.length).toBeGreaterThan(0);
    expect(() => applyVitestV5Migration(plan)).toThrow('blocking');
    expect(() => applyVitestV5NodeMigration(plan)).toThrow('blocking');
    expect(fs.readFileSync(path.join(root, '.node-version'), 'utf8')).toBe('20.19.0\n');
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toBe(
      'export default { test: {} };',
    );
    expect(fs.existsSync(path.join(root, '.vite-plus'))).toBe(false);
  });

  it('upgrades nvmrc and package runtimes without changing public engines', () => {
    const root = project({
      '.nvmrc': '25',
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        engines: { node: '>=18' },
        devEngines: { runtime: { name: 'node', version: '22.12.0' } },
      }),
    });
    const plan = planProject(root);
    const findings = plan.findings.filter((finding) => finding.code === 'node-runtime');
    expect(findings).toHaveLength(1);
    expect(findings).toContainEqual(
      expect.objectContaining({
        severity: 'review',
        message: expect.stringContaining('public engine contract'),
      }),
    );
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, '.nvmrc'), 'utf8')).toBe('26.0.0');
    expect(JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'))).toMatchObject({
      engines: { node: '>=18' },
      devEngines: { runtime: { name: 'node', version: '22.18.0' } },
    });
  });

  it.each([
    ['20.19.0\n', '22.18.0\n'],
    ['22.12.0', '22.18.0'],
    ['24.10.0', '24.11.0'],
    ['25', '26.0.0'],
    ['  v20.19.0\r\n', '  v22.18.0\r\n'],
    ['22.19.0', '22.19.0'],
    ['24.11.0', '24.11.0'],
    ['26.1.0', '26.1.0'],
  ])('migrates the runtime pin %j to %j', (before, after) => {
    const root = project({
      '.node-version': before,
      'vite.config.ts': 'export default { test: {} };',
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, '.node-version'), 'utf8')).toBe(after);
    expect(planProject(root).changes).toEqual([]);
  });

  it('upgrades runtime pins before install without applying source or script edits', () => {
    const manifest =
      '\uFEFF{\r\n  "devDependencies": { "vitest": "4.1.11" },\r\n  "devEngines": { "runtime": { "name": "node", "version": "20.19.0" } },\r\n  "scripts": { "list": "vitest list" }\r\n}\r\n';
    const original = 'export default { test: {} };';
    const root = project({
      'package.json': manifest,
      'vite.config.ts': original,
      '.node-version': '20.19.0\n',
    });
    const plan = planProject(root);
    expect(applyVitestV5NodeMigration(plan)).toBe(2);
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toBe(original);
    expect(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).toBe(
      manifest.replace('20.19.0', '22.18.0'),
    );
    const refreshed = refreshVitestV5Migration(plan);
    applyVitestV5Migration(refreshed);
    expect(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).toBe(
      manifest
        .replace('20.19.0', '22.18.0')
        .replace('vitest list', 'vitest list --no-static-parse'),
    );
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toContain(
      'clearMocks: false',
    );
  });

  it('refuses stale runtime declarations before any early upgrade', () => {
    const root = project({ '.node-version': '20.19.0' });
    const plan = planProject(root);
    fs.writeFileSync(path.join(root, '.node-version'), '24.11.0');
    expect(() => applyVitestV5NodeMigration(plan)).toThrow('Migration input changed');
    expect(fs.readFileSync(path.join(root, '.node-version'), 'utf8')).toBe('24.11.0');
  });

  it('ignores Node declarations outside the supported project files', () => {
    const root = project({
      'vite.config.ts': 'export default { test: {} };',
      '.github/workflows/test.yml':
        'jobs:\n  test:\n    strategy:\n      matrix:\n        node: [20, 25]\n    steps:\n      - uses: actions/setup-node@v6\n        with:\n          node-version: ${{ matrix.node }}\n',
    });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    expect(plan.changes.some(({ file }) => file.endsWith('test.yml'))).toBe(false);
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it.each([
    ['.nvmrc', 'node'],
    ['.nvmrc', 'stable'],
    ['.nvmrc', 'lts/*'],
    ['.node-version', 'lts/*'],
    ['.node-version', 'latest'],
    ['.node-version', 'current'],
    [
      'package.json',
      JSON.stringify({ devEngines: { runtime: { name: 'node', version: 'latest' } } }),
    ],
    ['package.json', JSON.stringify({ volta: { node: 'lts' } })],
  ])('accepts a moving runtime alias in %s: %s', (file, content) => {
    expect(
      planProject(project({ [file]: content })).findings.filter(
        ({ code }) => code === 'node-runtime',
      ),
    ).toEqual([]);
  });

  it.each(['lts/iron', 'lts/-2', '${{ matrix.node }}', 'custom-node', '*'])(
    'keeps the runtime review for unresolved or broad selector %s',
    (alias) => {
      const findings = planProject(project({ '.node-version': alias })).findings.filter(
        ({ code }) => code === 'node-runtime',
      );
      expect(findings).toEqual([expect.objectContaining({ severity: 'review' })]);
    },
  );

  it.each([
    {
      runtime: { name: 'node', version: '20.19.0' },
      expected: { name: 'node', version: '22.18.0' },
    },
    {
      runtime: [
        { name: 'bun', version: '1.3.0' },
        { name: 'node', version: '25.9.0' },
      ],
      expected: [
        { name: 'bun', version: '1.3.0' },
        { name: 'node', version: '26.0.0' },
      ],
    },
  ])('upgrades Node versions in package.json devEngines.runtime: %j', ({ runtime, expected }) => {
    const root = project({ 'package.json': JSON.stringify({ devEngines: { runtime } }) });
    const plan = planProject(root);
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(
      JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).devEngines.runtime,
    ).toEqual(expected);
    expect(planProject(root).changes).toEqual([]);
  });

  it.each(['>= 22.19.0', '>=22.18.0', '>=24.11.0', '>=26.0.0', '^22.19.0 || >=24.11.0'])(
    'accepts public engines.node %s with a supported minimum',
    (node) => {
      const manifest = JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        engines: { node },
      });
      const root = project({
        'package.json': manifest,
        'vite.config.ts': 'export default { test: {} };',
      });
      const plan = planProject(root);
      expect(plan.findings).toEqual([]);
      applyVitestV5Migration(plan);
      expect(finishVitestV5Migration(plan)).toEqual([]);
      expect(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).toBe(manifest);
      expect(planProject(root).findings).toEqual([]);
    },
  );

  it.each(['lts/*', 'latest', '*', '>=18', '>=22.17.0', '>=24.10.0', '>=25', '20 || >=22.19'])(
    'reviews public engines.node %s without a supported minimum',
    (node) => {
      const root = project({ 'package.json': JSON.stringify({ engines: { node } }) });
      expect(planProject(root).findings.filter(({ code }) => code === 'node-runtime')).toEqual([
        expect.objectContaining({ severity: 'review' }),
      ]);
    },
  );

  it('keeps a library public engine contract separate from its test runtime pin', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        engines: { node: '>=22.19.0' },
      }),
      '.node-version': '25.9.0',
    });
    const plan = planProject(root);
    expect(plan.findings.filter(({ code }) => code === 'node-runtime')).toEqual([]);
    applyVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, '.node-version'), 'utf8')).toBe('26.0.0');
    expect(JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).engines.node).toBe(
      '>=22.19.0',
    );
  });

  it('upgrades Volta before its pin is migrated, but respects a higher-priority pin file', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        volta: { node: '20.19.0' },
      }),
    });
    const plan = planProject(root);
    expect(plan.findings.filter(({ code }) => code === 'node-runtime')).toEqual([]);
    expect(plan.changes.find(({ file }) => file.endsWith('package.json'))?.after).toContain(
      '22.18.0',
    );
    fs.writeFileSync(path.join(root, '.nvmrc'), '24.11.0');
    expect(planProject(root).changes).toEqual([]);
  });

  it('keeps configless projects configless without recording completion', () => {
    const root = project({ '.gitignore': '.vitest-reports/\n__screenshots__/\n' });
    const plan = planProject(root);
    expect(plan.findings.some((finding) => finding.code === 'configless-defaults')).toBe(true);
    applyVitestV5Migration(plan);
    expect(vitestV5NeedsMigration(plan)).toBe(true);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'configless-defaults' }),
    );
    expect(fs.existsSync(path.join(root, 'vite.config.ts'))).toBe(false);
    expect(fs.readFileSync(path.join(root, '.gitignore'), 'utf8')).toBe(
      '.vitest-reports/\n__screenshots__/\n.vitest/\n',
    );
    expect(fs.existsSync(path.join(root, '.vite-plus'))).toBe(false);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    const repeated = planProject(root);
    expect(repeated.findings).toEqual([]);
    expect(vitestV5NeedsMigration(repeated)).toBe(false);
    expect(vitestV5ConfiglessProjects(repeated)).toEqual([]);
    expect(fs.existsSync(path.join(root, 'vite.config.ts'))).toBe(false);
  });

  it('preserves defaults in a config created by another migration step', () => {
    const root = project();
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    fs.writeFileSync(
      path.join(root, 'vite.config.ts'),
      "import { defineConfig } from 'vite-plus'; export default defineConfig({});",
    );
    finishVitestV5Migration(plan);
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toContain(
      'clearMocks: false',
    );
  });

  it('offers a separate creation action and never overwrites an existing config', () => {
    const root = project();
    const plan = planProject(root);
    const file = createVitestV5CompatibilityConfig(plan, root);
    expect(fs.readFileSync(file, 'utf8')).toContain('clearMocks: false');
    expect(() => createVitestV5CompatibilityConfig(plan, root)).toThrow('already exists');
  });

  it('does not reapply compatibility defaults after the user adopts v5 defaults', () => {
    const root = project({ 'vite.config.ts': 'export default { test: {} };' });
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    fs.writeFileSync(path.join(root, 'vite.config.ts'), 'export default { test: {} };');
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.0' } }),
    );
    const next = planProject(root);
    expect(next.projects[0].options.preserveV4).toBe(false);
    expect(vitestV5NeedsMigration(next)).toBe(false);
    applyVitestV5Migration(next);
    finishVitestV5Migration(next);
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toBe(
      'export default { test: {} };',
    );
    expect(fs.existsSync(path.join(root, '.vite-plus'))).toBe(false);
  });

  it.each([
    'not valid JSON',
    JSON.stringify({ version: 1, vitest5: { '.': { sourceVersion: '4.1.11', configless: true } } }),
    JSON.stringify({ version: 99 }),
  ])('ignores a legacy migration file without changing it: %s', (legacy) => {
    const root = project({
      'package.json': JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
      'vite.config.ts': 'export default { test: {} };',
      '.vite-plus/migrations.json': legacy,
    });
    const plan = planProject(root);
    expect(plan.projects[0].sourceVersion).toBe('5.0.1');
    expect(plan.findings).toEqual([]);
    expect(plan.changes).toEqual([]);
    expect(vitestV5NeedsMigration(plan)).toBe(false);
    expect(plan.inputs.has(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toEqual([]);
    expect(fs.readFileSync(path.join(root, '.vite-plus/migrations.json'), 'utf8')).toBe(legacy);
  });

  it('does not let a legacy completion record skip a resolved v4 migration', () => {
    const root = project({
      'vite.config.ts': 'export default { test: {} };',
      '.vite-plus/migrations.json': JSON.stringify({
        version: 1,
        vitest5: { '.': { sourceVersion: '5.0.1', configless: false } },
      }),
    });
    const plan = planProject(root);
    expect(plan.projects[0].options.preserveV4).toBe(true);
    expect(plan.changes[0].after).toContain('clearMocks: false');
    expect(vitestV5NeedsMigration(plan)).toBe(true);
  });

  it('still blocks unresolved runner versions instead of trusting a legacy record', () => {
    const root = project({
      'package.json': '{}',
      'unit.test.ts': "import { test } from 'vite-plus/test'; test('works', () => {});",
      '.vite-plus/migrations.json': JSON.stringify({
        version: 1,
        vitest5: { '.': { sourceVersion: '4.1.11', configless: false } },
      }),
    });
    expect(planProject(root).findings).toContainEqual(
      expect.objectContaining({ code: 'source-version', severity: 'block' }),
    );
  });

  it('resolves v4 catalogs and leaves already-v5 projects on new defaults', () => {
    const root = project({
      'package.json': JSON.stringify({ devDependencies: { vitest: 'catalog:testing' } }),
      'pnpm-workspace.yaml': 'catalogs:\n  testing:\n    vitest: ^4.1.11\n',
      'vite.config.ts': 'export default { test: {} };',
    });
    expect(planProject(root).projects[0].sourceVersion).toBe('4.1.11');
    const v5root = project({
      'package.json': JSON.stringify({ devDependencies: { vitest: '5.0.0' } }),
      'vite.config.ts': 'export default { test: {} };',
    });
    expect(planProject(v5root).changes).toEqual([]);
  });

  it('retains unresolved findings on later scans and groups them by file', () => {
    const root = project({
      'mock.test.ts': "import { test, vi } from 'vitest'; test('mock', () => vi.mock('./a'));",
    });
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    const after = finishVitestV5Migration(plan);
    expect(after.some((finding) => finding.code === 'nested-hoisted-mock')).toBe(true);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    expect(
      planProject(root).findings.some((finding) => finding.code === 'nested-hoisted-mock'),
    ).toBe(true);
    const report = formatVitestV5Findings(plan);
    expect(report).toContain('Vitest v5:');
    expect(report).toContain('mock.test.ts\n  1:');
  });

  it('retains deferred reviews in the final report without storing them for later v5 runs', () => {
    const input = `import { resolveConfig } from 'vitest/node';
const pair = await resolveConfig();
export const config = pair.viteConfig;`;
    const root = project({ 'runner.ts': input });
    const plan = planProject(root);
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'resolve-config' }));
    applyVitestV5Migration(plan);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'resolve-config' }),
    );
    const repeated = planProject(root);
    expect(repeated.findings.some(({ code }) => code === 'resolve-config')).toBe(false);
    expect(repeated.changes).toEqual([]);
    expect(fs.readFileSync(path.join(root, 'runner.ts'), 'utf8')).toBe(input);
    expect(fs.existsSync(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
    fs.writeFileSync(path.join(root, 'runner.ts'), 'export const config = {};');
    expect(planProject(root).findings.some(({ code }) => code === 'resolve-config')).toBe(false);
  });

  it('does not retain a resolveConfig warning for a successful automatic rewrite', () => {
    const root = project({
      'runner.ts': `import { resolveConfig } from 'vitest/node';\nconst { viteConfig } = await resolveConfig();`,
    });
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan).some(({ code }) => code === 'resolve-config')).toBe(false);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    expect(planProject(root).findings.some(({ code }) => code === 'resolve-config')).toBe(false);
  });

  it('resolves each workspace package independently on a rerun without state', () => {
    const root = project({
      'packages/a/package.json': JSON.stringify({ devDependencies: { vitest: '4.1.11' } }),
      'packages/a/vite.config.ts': 'export default { test: {} };',
    });
    const workspace = {
      rootDir: root,
      packageManager: PackageManager.pnpm,
      packages: [{ name: 'a', path: 'packages/a' }],
    };
    const plan = planVitestV5Migration(workspace);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    fs.writeFileSync(
      path.join(root, 'package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    const repeated = planVitestV5Migration(workspace);
    expect(repeated.projects.map(({ options }) => options.preserveV4)).toEqual([false, true]);
    fs.writeFileSync(
      path.join(root, 'packages/a/package.json'),
      JSON.stringify({ devDependencies: { vitest: '5.0.1' } }),
    );
    const upgraded = planVitestV5Migration(workspace);
    expect(upgraded.projects.map(({ options }) => options.preserveV4)).toEqual([false, false]);
    expect(vitestV5NeedsMigration(upgraded)).toBe(false);
    expect(fs.existsSync(path.join(root, '.vite-plus/migrations.json'))).toBe(false);
  });

  it('refuses to overwrite files changed since preflight', () => {
    const root = project({ 'vite.config.ts': 'export default { test: {} };' });
    const plan = planProject(root);
    fs.writeFileSync(path.join(root, 'vite.config.ts'), '// changed by editor\nexport default {};');
    expect(() => applyVitestV5Migration(plan)).toThrow('Migration input changed');
  });

  it('does not follow symlinks or edit fixtures under nested package boundaries', () => {
    const root = project({
      'nested/package.json': '{}',
      'nested/vite.config.ts': 'export default { test: {} };',
      'node_modules/ignored/vite.config.ts': 'export default { test: {} };',
    });
    expect(planProject(root).changes).toEqual([]);
  });
});

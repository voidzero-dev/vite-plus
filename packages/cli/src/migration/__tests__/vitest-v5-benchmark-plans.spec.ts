import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  applyVitestV5Migration,
  applyVitestV5NodeMigration,
  finishVitestV5Migration,
  planVitestV5Migration,
} from '../migrator.ts';
import { SourceEditor } from '../vitest-v5/ast.ts';

const directories: string[] = [];
afterEach(() => {
  vi.restoreAllMocks();
  for (const directory of directories.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

function project(files: Record<string, string>) {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-benchmark-plan-'));
  directories.push(rootDir);
  for (const [name, source] of Object.entries({
    'package.json': JSON.stringify({
      devDependencies: { vitest: '4.1.11' },
      scripts: { bench: 'vitest bench --run' },
    }),
    ...files,
  })) {
    const file = path.join(rootDir, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, source);
  }
  return { rootDir, packageManager: PackageManager.pnpm };
}

describe('Vitest v5 benchmark migration plans', () => {
  it.each([
    ['{ globals: true }', 'example.bench.js'],
    ['{ globals: true }', 'example.benchmark.ts'],
    [
      `{ globals: true, include: ['unit/*.test.js'], exclude: ['perf/**'], benchmark: { include: ['perf/*.js'], exclude: ['**/unrelated.js'] } }`,
      'perf/work.js',
    ],
    [
      `{ globals: true, root: './app', dir: './perf', benchmark: { include: ['*.js'], exclude: ['unrelated.js'] } }`,
      'perf/work.js',
    ],
    [
      `{ projects: [{ test: { root: './perf', globals: true, benchmark: { include: ['*.js'], exclude: ['unrelated.js'] } } }] }`,
      'perf/work.js',
    ],
    [
      `{ globals: true, projects: [{ extends: true, test: { benchmark: { include: ['perf/*.js'], exclude: ['**/unrelated.js'] } } }] }`,
      'perf/work.js',
    ],
  ])('resolves benchmark ownership from %s', (test, selected) => {
    const unrelated = `bench('other runner', () => { expect(() => {}).toThrow(''); });`;
    const workspace = project({
      'vitest.config.mjs': `export default { test: ${test} };`,
      [selected]: `bench('work', (() => 42));`,
      'perf/unrelated.js': unrelated,
      'unrelated.js': unrelated,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toEqual([]);
    const changed = plan.changes.find(
      ({ file }) => file === path.join(workspace.rootDir, selected),
    );
    expect(changed?.after).toContain(`globalThis.test('work'`);
    expect(changed?.after).toContain('async ({ bench })');
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toEqual([]);
    for (const file of ['perf/unrelated.js', 'unrelated.js']) {
      expect(fs.readFileSync(path.join(workspace.rootDir, file), 'utf8')).toBe(unrelated);
    }
    expect(planVitestV5Migration(workspace).changes).toEqual([]);
  });

  it('merges explicit inherited benchmark patterns without inheriting implicit defaults', () => {
    const workspace = project({
      'vitest.config.mjs': `export default { test: { globals: true, benchmark: { include: ['base/*.js'], exclude: ['**/excluded.js'] }, projects: [{ extends: true, test: { benchmark: { include: ['perf/*.js'], exclude: ['**/other.js'] } } }] } };`,
      'base/work.js': `bench('base', () => 42);`,
      'perf/work.js': `bench('child', () => 42);`,
      'base/excluded.js': `bench('excluded', () => 42);`,
      'perf/other.js': `bench('other', () => 42);`,
      'unused.bench.js': `bench('default pattern is overridden', () => 42);`,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toEqual([]);
    expect(plan.changes.map(({ file }) => path.relative(workspace.rootDir, file))).toEqual([
      path.join('base', 'work.js'),
      path.join('perf', 'work.js'),
      'vitest.config.mjs',
    ]);
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it('uses benchmark settings from referenced projects and their roots', () => {
    const workspace = project({
      'vitest.config.mjs': `export default { test: { projects: ['./perf/vitest.config.mjs'] } };`,
      'perf/vitest.config.mjs': `export default { test: { globals: true, benchmark: { include: ['*.js'], exclude: ['unrelated.js'] } } };`,
      'perf/work.js': `bench('work', () => 42);`,
      'perf/unrelated.js': `bench('unrelated', () => 42);`,
      'work.js': `bench('wrong root', () => 42);`,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toEqual([]);
    expect(plan.changes.filter(({ file }) => file.endsWith('.js')).map(({ file }) => file)).toEqual(
      [path.join(workspace.rootDir, 'perf/work.js')],
    );
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it('does not merge implicit parent benchmark defaults into an inline project', () => {
    const workspace = project({
      'vitest.config.mjs': `export default { test: { globals: true, projects: [{ extends: true, test: { benchmark: { include: ['perf/*.js'] } } }] } };`,
      'perf/work.js': `bench('work', () => 42);`,
      'unused.bench.js': `bench('other runner', () => 42);`,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toEqual([]);
    expect(plan.changes.filter(({ file }) => file.endsWith('.js')).map(({ file }) => file)).toEqual(
      [path.join(workspace.rootDir, 'perf/work.js')],
    );
  });

  it('includes in-source benchmarks only when the file contains import.meta.vitest', () => {
    const workspace = project({
      'vitest.config.mjs': `export default { test: { globals: true, benchmark: { include: [], includeSource: ['perf/*.js'] } } };`,
      'perf/work.js': `if (import.meta.vitest) { bench('work', () => 42); }`,
      'perf/other.js': `bench('other runner', () => 42);`,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toEqual([]);
    expect(plan.changes.filter(({ file }) => file.endsWith('.js')).map(({ file }) => file)).toEqual(
      [path.join(workspace.rootDir, 'perf/work.js')],
    );
  });

  it('reviews CLI benchmark pattern overrides instead of using the config patterns', () => {
    const workspace = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        scripts: { bench: 'vitest bench --benchmark.include=perf/*.js' },
      }),
      'vitest.config.mjs': `export default { test: { globals: true } };`,
      'perf/work.js': `bench('work', () => 42);`,
      'unused.bench.js': `bench('other runner', () => 42);`,
    });
    const plan = planVitestV5Migration(workspace);
    for (const file of ['perf/work.js', 'unused.bench.js']) {
      expect(plan.findings).toContainEqual(
        expect.objectContaining({
          file: path.join(workspace.rootDir, file),
          code: 'global-api-ownership',
        }),
      );
    }
    expect(plan.changes.some(({ file }) => file.endsWith('.js'))).toBe(false);
  });

  describe.each([false, true])('in-source plans with globals: %s', (globals) => {
    it.each([
      `const { bench } = import.meta.vitest; bench('work', () => work());`,
      `import.meta.vitest.bench('work', () => work());`,
    ])('scans, applies, and finalizes %s', (body) => {
      const workspace = project({
        'vitest.config.mjs': `export default { test: { globals: ${globals}, benchmark: { include: [], includeSource: ['perf/*.js'] } } };`,
        'perf/work.js': `export const work = () => 42; if (import.meta.vitest) { ${body} }`,
        'perf/other.js': `bench('other runner', () => 42);`,
      });
      const plan = planVitestV5Migration(workspace);
      expect(plan.findings).toEqual([]);
      const content = plan.changes.find(({ file }) => file.endsWith('work.js'))?.after;
      expect(content).toContain('export const work = () => 42; if (import.meta.vitest)');
      expect(content).toContain(`test('work', async ({ bench })`);
      expect(content).not.toMatch(/\bimport\s*[{*]/);
      applyVitestV5Migration(plan);
      expect(finishVitestV5Migration(plan)).toEqual([]);
      expect(fs.readFileSync(path.join(workspace.rootDir, 'perf/other.js'), 'utf8')).toBe(
        `bench('other runner', () => 42);`,
      );
      expect(planVitestV5Migration(workspace).changes).toEqual([]);
    });

    it.each([
      `const { bench } = import.meta.vitest; bench('work', () => 42, { time: 1 });`,
      `import.meta.vitest.bench('work', () => 42, { time: 1 });`,
      `const { bench } = import.meta.vitest; register(bench);`,
    ])('blocks dependency changes for retained APIs: %s', (body) => {
      const original = `if (import.meta.vitest) { ${body} }`;
      const workspace = project({
        'vitest.config.mjs': `export default { test: { globals: ${globals}, benchmark: { include: [], includeSource: ['perf/*.js'] } } };`,
        'perf/work.js': original,
      });
      const manifest = fs.readFileSync(path.join(workspace.rootDir, 'package.json'), 'utf8');
      const plan = planVitestV5Migration(workspace);
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
      );
      expect(() => applyVitestV5Migration(plan)).toThrow('blocking');
      expect(fs.readFileSync(path.join(workspace.rootDir, 'package.json'), 'utf8')).toBe(manifest);
      expect(fs.readFileSync(path.join(workspace.rootDir, 'perf/work.js'), 'utf8')).toBe(original);
      expect(finishVitestV5Migration(plan)).toContainEqual(
        expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
      );
    });
  });

  it.each([
    'runtimeOptions',
    '{ include: runtimePatterns }',
    '{ exclude: runtimePatterns }',
    `{ includeSource: runtimePatterns }`,
  ])('reviews unresolved benchmark membership: %s', (benchmark) => {
    const original = `bench('work', () => 42);`;
    const workspace = project({
      'vitest.config.mjs': `export default { test: { globals: true, benchmark: ${benchmark} } };`,
      'example.bench.js': original,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        file: path.join(workspace.rootDir, 'example.bench.js'),
        code: 'global-api-ownership',
      }),
    );
    expect(plan.changes.some(({ file }) => file.endsWith('example.bench.js'))).toBe(false);
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'global-api-ownership' }),
    );
    expect(fs.readFileSync(path.join(workspace.rootDir, 'example.bench.js'), 'utf8')).toBe(
      original,
    );
  });

  it('reviews conflicting benchmark globals across projects', () => {
    const workspace = project({
      'vitest.config.mjs': `export default { test: { projects: [{ test: { globals: true } }, { test: { globals: false } }] } };`,
      'example.bench.js': `bench('shared', () => 42);`,
    });
    const plan = planVitestV5Migration(workspace);
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
    expect(plan.changes.some(({ file }) => file.endsWith('example.bench.js'))).toBe(false);
  });

  it.each(['(() => 42)', '((() => 42))', '(function () { return 42; })', '(workload)'])(
    'applies and finalizes a parenthesized imported benchmark: %s',
    (callback) => {
      const workspace = project({
        'vitest.config.mjs': 'export default { test: {} };',
        'example.bench.js': `import { bench } from 'vitest'; const workload = () => 42; bench('work', ${callback});`,
      });
      const plan = planVitestV5Migration(workspace);
      expect(plan.findings).toEqual([]);
      expect(plan.changes.find(({ file }) => file.endsWith('example.bench.js'))?.after).toContain(
        "test('work'",
      );
      applyVitestV5Migration(plan);
      expect(finishVitestV5Migration(plan)).toEqual([]);
      expect(planVitestV5Migration(workspace).changes).toEqual([]);
    },
  );

  describe.each(['unsafe-syntax', 'overlapping-edits'])('rollback after %s', (code) => {
    it.each([
      `import { bench } from 'vitest'; bench('work', () => 42);`,
      `import * as v from 'vitest'; v.bench('work', () => 42);`,
      `bench('work', () => 42);`,
      `import { bench } from 'vitest';`,
      `if (import.meta.vitest) { const { bench } = import.meta.vitest; bench('work', () => 42); }`,
      `if (import.meta.vitest) { import.meta.vitest.bench('work', () => 42); }`,
      `if (import.meta.vitest) { const { bench } = import.meta.vitest; }`,
    ])('blocks the plan while the legacy API remains: %s', (original) => {
      const workspace = project({
        '.node-version': '20.19.0\n',
        'vitest.config.mjs': 'export default { test: { globals: true } };',
        'example.bench.js': original,
      });
      // oxlint-disable-next-line typescript/unbound-method -- Invoked below with the current editor as this.
      const finish = SourceEditor.prototype.finish;
      // Exercise the real validation and rollback paths independently of the
      // particular transform bug that caused a rejected edit.
      vi.spyOn(SourceEditor.prototype, 'finish').mockImplementation(function (this: SourceEditor) {
        if (this.file.endsWith('example.bench.js')) {
          if (code === 'unsafe-syntax') {
            this.edit(this.source.length, this.source.length, '\nconst =;');
          } else {
            this.edit(0, this.source.length, '');
          }
        }
        return finish.call(this);
      });
      const manifest = fs.readFileSync(path.join(workspace.rootDir, 'package.json'), 'utf8');
      const plan = planVitestV5Migration(workspace);
      expect(plan.findings).toContainEqual(expect.objectContaining({ code }));
      expect(plan.findings).toContainEqual(
        expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
      );
      expect(plan.changes.some(({ file }) => file.endsWith('example.bench.js'))).toBe(false);
      expect(() => applyVitestV5Migration(plan)).toThrow('blocking');
      expect(() => applyVitestV5NodeMigration(plan)).toThrow('blocking');
      expect(fs.readFileSync(path.join(workspace.rootDir, 'example.bench.js'), 'utf8')).toBe(
        original,
      );
      expect(fs.readFileSync(path.join(workspace.rootDir, 'package.json'), 'utf8')).toBe(manifest);
      expect(fs.readFileSync(path.join(workspace.rootDir, '.node-version'), 'utf8')).toBe(
        '20.19.0\n',
      );
      expect(finishVitestV5Migration(plan)).toContainEqual(
        expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
      );
    });
  });
});

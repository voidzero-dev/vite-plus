import { mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

import { parseAst } from 'vite/rolldown/parseAst';
import { afterEach, describe, expect, it } from 'vitest';

import {
  analyzeModule,
  bundledSet,
  metaReplacement,
  startupLayout,
} from '../script-bundle-plan.ts';
import {
  BUNDLE_CACHE_VERSION,
  type BundleCacheContext,
  bundleKey,
  changedInput,
  checkManifest,
  findCacheRoot,
  isPlainDependency,
  isPlainDependencyImport,
  MANIFEST,
  parseResolveFrom,
  stamp,
  unsupportedReason,
  writeJsonAtomic,
} from '../script-bundle.ts';

const dirs: string[] = [];

function project(files: Record<string, string>): string {
  const dir = mkdtempSync(path.join(tmpdir(), 'vpx-bundle-'));
  dirs.push(dir);
  for (const [name, content] of Object.entries(files)) {
    const file = path.join(dir, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, content);
  }
  return dir;
}

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

function analyze(code: string) {
  return analyzeModule(parseAst(code, { lang: 'js', sourceType: 'module' }) as never);
}

describe('analyzeModule', () => {
  it('collects static, re-exported, and literal dynamic specifiers', () => {
    const analysis = analyze(
      [
        "import a from './a.js';",
        "import './side-effect.js';",
        "export { b } from './b.js';",
        "export * from 'dep';",
        "const c = await import('./c.js');",
        'const d = await import(`./d.js`);',
      ].join('\n'),
    );
    expect(analysis.staticSpecifiers).toEqual(['./a.js', './side-effect.js', './b.js', 'dep']);
    expect(analysis.dynamicImports.map((entry) => entry.specifier)).toEqual(['./c.js', './d.js']);
    expect(analysis.unsafe).toBeUndefined();
    expect(analysis.resolveEdits).toEqual([]);
  });

  it('marks import.meta reads it can replace', () => {
    const code =
      'console.log(import.meta.url, import.meta.filename, import.meta.dirname, import.meta.main);';
    const analysis = analyze(code);
    expect(analysis.unsafe).toBeUndefined();
    expect(
      analysis.metaEdits.map((edit) => [edit.property, code.slice(edit.start, edit.end)]),
    ).toEqual([
      ['url', 'import.meta.url'],
      ['filename', 'import.meta.filename'],
      ['dirname', 'import.meta.dirname'],
      ['main', 'import.meta.main'],
    ]);
  });

  it('marks the specifiers only the run knows', () => {
    const code = "const m = await import(name + '.js'); const u = import.meta.resolve(dep);";
    const analysis = analyze(code);
    expect(analysis.unsafe).toBeUndefined();
    expect(analysis.resolveEdits.map((edit) => code.slice(edit.start, edit.end))).toEqual([
      "name + '.js'",
      'dep',
    ]);
  });

  it.each([
    ['await ready;', true],
    ['for await (const x of xs) {}', true],
    ['await using handle = open();', true],
    ['async function f() { await ready; }', false],
    ['const f = async () => { for await (const x of xs) {} };', false],
  ])('detects top-level await in %s', (code, expected) => {
    expect(analyze(code).topLevelAwait).toBe(expected);
  });

  it.each([
    ['const meta = import.meta;', 'import.meta'],
    ['console.log(import.meta.env);', 'import.meta.env'],
    ['const resolve = import.meta.resolve;', 'import.meta.resolve'],
    ["eval('1');", 'a direct eval()'],
  ])('leaves out %s', (code, reason) => {
    expect(analyze(code).unsafe).toBe(reason);
  });
});

describe('metaReplacement', () => {
  it('replaces each read with the module’s own value', () => {
    const url = 'file:///p/src/a.ts';
    expect(metaReplacement('url', url, '/p/src/a.ts', '/p/src', false)).toBe(JSON.stringify(url));
    expect(metaReplacement('filename', url, '/p/src/a.ts', '/p/src', false)).toBe('"/p/src/a.ts"');
    expect(metaReplacement('dirname', url, '/p/src/a.ts', '/p/src', false)).toBe('"/p/src"');
  });

  it('keeps import.meta.main only for the entry', () => {
    expect(metaReplacement('main', 'file:///p/a.ts', '/p/a.ts', '/p', true)).toBeUndefined();
    expect(metaReplacement('main', 'file:///p/b.ts', '/p/b.ts', '/p', false)).toBe(
      '(import.meta.main && false)',
    );
  });
});

function graph(modules: Record<string, { imports: string[]; unsafe?: string }>) {
  return new Map(Object.entries(modules));
}

describe('bundledSet', () => {
  it('bundles the safe ES modules reachable from the entry', () => {
    const modules = graph({
      entry: { imports: ['a', 'cjs'] },
      a: { imports: ['b'] },
      b: { imports: [] },
      unreachable: { imports: [] },
    });
    expect(bundledSet('entry', modules, [], new Set())).toEqual(new Set(['a', 'b', 'entry']));
  });

  it('leaves out every module that code outside the bundle imports, and what it imports', () => {
    const modules = graph({
      entry: { imports: ['a', 'unsafe', 'shared'] },
      a: { imports: ['shared'] },
      unsafe: { unsafe: 'import.meta', imports: ['shared'] },
      shared: { imports: ['leaf'] },
      leaf: { imports: [] },
    });
    expect(bundledSet('entry', modules, [], new Set())).toEqual(new Set(['a', 'entry']));
  });

  it('applies run edges and exclusions', () => {
    const modules = graph({
      entry: { imports: ['a', 'b'] },
      a: { imports: [] },
      b: { imports: [] },
    });
    // A `require()` of `a` in the run, and `b` reported as loaded twice.
    expect([...bundledSet('entry', modules, [['', 'a']], new Set(['b']))]).toEqual(['entry']);
  });

  it('bundles nothing when the entry cannot be bundled', () => {
    const modules = graph({ entry: { imports: [] } });
    expect(bundledSet('entry', modules, [['', 'entry']], new Set()).size).toBe(0);
    expect(bundledSet('other', modules, [], new Set()).size).toBe(0);
  });
});

describe('startupLayout', () => {
  it('keeps the modules only import() reaches out of the startup code', () => {
    const staticImports = new Map([
      ['entry', ['shared']],
      ['lazy', ['shared', 'lazy-only']],
    ]);
    const bundled = new Set(['entry', 'shared', 'lazy', 'lazy-only']);
    const layout = startupLayout('entry', staticImports, bundled);
    expect(layout.modules).toEqual(new Set(['shared', 'entry']));
    expect(layout.segments).toEqual([['shared']]);
    expect(layout.externals).toEqual([]);
    expect(startupLayout('missing', staticImports, bundled).modules.size).toBe(0);
  });

  it('splits the startup code at each external module, in the order Node.js runs it', () => {
    // entry: a, cjs1, b, esm1, c; c: cjs2, a; built-ins have no place in the order.
    const staticImports = new Map([
      ['entry', ['a', 'file:///cjs1', 'b', 'file:///esm1', 'c', 'node:fs']],
      ['c', ['file:///cjs2', 'a']],
    ]);
    const layout = startupLayout('entry', staticImports, new Set(['entry', 'a', 'b', 'c']));
    expect(layout.externals).toEqual(['file:///cjs1', 'file:///esm1', 'file:///cjs2']);
    expect(layout.segments).toEqual([['a'], ['b'], ['c']]);
  });
});

describe('parseResolveFrom', () => {
  it('splits the module index from the specifier', () => {
    expect(parseResolveFrom('vpx-from:12:./plugins/a.ts')).toEqual([12, './plugins/a.ts']);
    expect(parseResolveFrom('vpx-from:3:file:///p/a.js')).toEqual([3, 'file:///p/a.js']);
    expect(parseResolveFrom('./a.ts')).toBeUndefined();
    expect(parseResolveFrom('vpx-from:x:./a.ts')).toBeUndefined();
  });
});

describe('unsupportedReason', () => {
  const own = { preload: '/vp/dist/script-preload.cjs', register: '/vp/dist/script-register.js' };
  const loader = ['--require', own.preload, '--import', pathToFileURL(own.register).href];

  it('accepts the loader’s own entries and other options', () => {
    expect(unsupportedReason({}, [...loader, '--conditions', 'dev'], own)).toBeUndefined();
  });

  it.each([
    [{}, ['--require', './setup.ts'], '--require loads other modules or hooks'],
    [{}, ['--import=tsx'], '--import loads other modules or hooks'],
    [{ NODE_OPTIONS: '--import ./x.mjs' }, [], 'NODE_OPTIONS loads other modules or hooks'],
    [{ NODE_V8_COVERAGE: '/tmp/coverage' }, [], 'NODE_V8_COVERAGE is set'],
    [{}, ['--test'], '--test is not supported'],
    [
      {},
      ['--experimental-config-file=node.config.json'],
      '--experimental-config-file is not supported',
    ],
  ])('rejects %j %j', (env, execArgv, reason) => {
    expect(unsupportedReason(env, [...loader, ...execArgv], own)).toBe(reason);
  });
});

describe('cache files', () => {
  it('finds the nearest node_modules above the script', () => {
    const dir = project({ 'node_modules/.keep': '', 'scripts/deep/a.ts': '' });
    expect(findCacheRoot(path.join(dir, 'scripts/deep/a.ts'))).toBe(
      path.join(dir, 'node_modules/.vite-plus/vpx-bundle'),
    );
    expect(findCacheRoot('/no-project-here/a.ts')).toBeUndefined();
  });

  it('detects an edit that restores the size and the modification time', () => {
    const dir = project({ 'a.ts': 'one' });
    const file = path.join(dir, 'a.ts');
    const before = stamp(file);
    const time = new Date('2026-01-01T00:00:00Z');
    utimesSync(file, time, time);
    const restored = stamp(file);
    writeFileSync(file, 'two');
    utimesSync(file, time, time);
    expect(stamp(file)).not.toBe(restored);
    expect(changedInput([[file, before]])).toBe(file);
    expect(changedInput([[path.join(dir, 'missing.ts'), '']])).toBeUndefined();
  });

  it('reports a hit, a disabled entry, and a miss', () => {
    const dir = project({ 'a.ts': '' });
    const file = path.join(dir, 'a.ts');
    const context: BundleCacheContext = {
      entryPath: file,
      entryURL: pathToFileURL(file).href,
      cwd: dir,
      key: bundleKey(file, dir, {}, []),
      root: dir,
      dir: path.join(dir, 'cache'),
    };
    const write = (manifest: object) =>
      writeJsonAtomic(path.join(context.dir, MANIFEST), {
        version: BUNDLE_CACHE_VERSION,
        key: context.key,
        inputs: [[file, stamp(file)]],
        ...manifest,
      });
    expect(checkManifest(context)).toEqual({ kind: 'miss', reason: 'no bundle' });
    write({ out: 'out-1', entry: 'entry.mjs', chunks: ['entry.mjs', 'chunk-a.mjs'], modules: [] });
    const out = path.join(context.dir, 'out-1');
    mkdirSync(out, { recursive: true });
    writeFileSync(path.join(out, 'entry.mjs'), '');
    expect(checkManifest(context)).toEqual({
      kind: 'miss',
      reason: `${path.join(out, 'chunk-a.mjs')} is missing`,
    });
    writeFileSync(path.join(out, 'chunk-a.mjs'), '');
    expect(checkManifest(context).kind).toBe('hit');
    write({ disabled: 'the entry is not an ES module' });
    expect(checkManifest(context)).toEqual({
      kind: 'disabled',
      reason: 'the entry is not an ES module',
    });
    write({ key: 'another', out: 'out-1', entry: 'entry.mjs' });
    expect(checkManifest(context)).toEqual({ kind: 'miss', reason: 'another configuration' });
    write({ out: 'out-1', entry: 'entry.mjs' });
    writeFileSync(file, 'edited');
    expect(checkManifest(context)).toEqual({ kind: 'miss', reason: `${file} changed` });
  });
});

describe('dependency fast paths', () => {
  it('treats JavaScript in node_modules as Node.js resolves and loads it', () => {
    expect(isPlainDependency('file:///p/node_modules/dep/index.js')).toBe(true);
    expect(isPlainDependency('file:///p/node_modules/dep/index.ts')).toBe(false);
    expect(isPlainDependency('file:///p/node_modules/dep/data.json')).toBe(false);
    expect(isPlainDependency('file:///p/src/index.js')).toBe(false);
  });

  it('takes a relative import in a dependency only when the named file exists', () => {
    const dir = project({
      'node_modules/dep/index.js': '',
      'node_modules/dep/lib.js': '',
      'node_modules/dep/types.ts': '',
    });
    const parent = pathToFileURL(path.join(dir, 'node_modules/dep/index.js')).href;
    expect(isPlainDependencyImport('./lib.js', parent)).toBe(true);
    expect(isPlainDependencyImport('./lib', parent)).toBe(false);
    expect(isPlainDependencyImport('./missing.js', parent)).toBe(false);
    expect(isPlainDependencyImport('./types.ts', parent)).toBe(false);
    expect(isPlainDependencyImport('dep2', parent)).toBe(false);
  });
});

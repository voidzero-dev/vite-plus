import { runInNewContext } from 'node:vm';

import { describe, expect, it } from 'vitest';

import { migrateVitestV5Command } from '../vitest-v5/commands.ts';
import { migrateVitestV5Source } from '../vitest-v5/source.ts';

const migrate = (source: string, globals = false) =>
  migrateVitestV5Source('example.bench.ts', source, { preserveV4: true, globals });

describe('Vitest v5 benchmark migration', () => {
  it.each(['vitest', 'vite-plus/test'])(
    'migrates direct calls imported from %s and preserves workload scopes',
    (module) => {
      const input = `import { bench as measure, describe } from '${module}';
describe('utilities', () => {
  const input = '{"a":1}';
  measure('parse', () => { JSON.parse(input); });
  measure('async', async () => { await Promise.resolve(input); });
});`;
      const result = migrate(input);
      expect(result.findings).toEqual([]);
      expect(result.content).toContain('import { test, describe }');
      expect(result.content).toContain(
        `test('parse', async ({ bench }) => { await bench('parse', () => { JSON.parse(input); }).run(); });`,
      );
      expect(result.content).toContain(
        `await bench('async', async () => { await Promise.resolve(input); }).run();`,
      );
      expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
    },
  );

  it.each(['skip', 'only', 'todo'])('moves bench.%s to the enclosing test', (modifier) => {
    const result = migrate(
      `import { bench } from 'vitest'; bench.${modifier}('case', () => work());`,
    );
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`test.${modifier}('case', async ({ bench })`);
    expect(result.content).toContain(`await bench('case', () => work()).run();`);
  });

  it('migrates a todo with no workload', () => {
    const result = migrate(`import { bench } from 'vitest'; bench.todo('later');`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`test.todo('later');`);
    expect(result.content).not.toContain('.run()');
  });

  it('migrates namespace imports and preserves an existing fixture', () => {
    const result = migrate(`import * as v from 'vitest';
v.bench('old', () => work());
v.test('new', async ({ bench }) => { await bench('new', () => work()).run(); });`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`v.test('old', async ({ bench })`);
    expect(result.content).toContain(
      `v.test('new', async ({ bench }) => { await bench('new', () => work()).run(); });`,
    );
    expect(migrate(result.content).content).toBe(result.content);
  });

  it('avoids collisions with existing test and bench identifiers', () => {
    const result = migrate(`import { bench as measure, test } from 'vitest';
const _test = 1, _bench = 2, bench = 3;
measure('scope', function () { consume(_test, _bench, test, bench); });`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('test as _test2');
    expect(result.content).toContain('bench: _bench2');
    expect(result.content).toContain('function () { consume(_test, _bench, test, bench); }');
  });

  it('avoids a test binding inside the registration scope', () => {
    const result = migrate(`import { bench, describe } from 'vitest';
describe('group', () => {
  const test = () => 42;
  bench('scope', () => test());
});`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('import { test as _test, describe }');
    expect(result.content).toContain(`_test('scope', async ({ bench })`);
    expect(result.content).toContain(`await bench('scope', () => test()).run()`);
    expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
  });

  it('reserves generated test imports across separate benchmark imports', () => {
    const result = migrate(`import { bench as first } from 'vitest';
import { bench as second } from 'vite-plus/test';
first('first', () => work());
second('second', () => work());`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`import { test } from 'vitest';`);
    expect(result.content).toContain(`import { test as _test } from 'vite-plus/test';`);
    expect(result.content).toContain(`test('first', async ({ bench })`);
    expect(result.content).toContain(`_test('second', async ({ bench })`);
    expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
  });

  it('uses a plain fixture for a named workload that closes over bench', () => {
    const result = migrate(`import { bench as measure } from 'vitest';
const bench = 42;
function workload() { consume(bench); }
measure('scope', workload);`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`import { test } from 'vitest';`);
    expect(result.content).toContain('function workload() { consume(bench); }');
    expect(result.content).toContain('const _benchFn = workload;');
    expect(result.content).toContain(`test('scope', async ({ bench })`);
    expect(result.content).toContain(`await bench('scope', _benchFn).run()`);
    expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
  });

  it.each(['const bench = 42;', ''])(
    'preserves bound and unbound workload references when adding aliases: %s',
    async (declaration) => {
      const result = migrate(`import { bench as measure } from 'vitest';
${declaration}
measure('scope', () => results.push([test, bench]));
measure('plain', () => results.push('plain'));`);
      expect(result.findings).toEqual([]);
      expect(result.content).toContain(`import { test as _test } from 'vitest';`);
      expect(result.content).toContain(`_test('scope', async ({ bench: _bench })`);
      expect(result.content).toContain(`_test('plain', async ({ bench })`);
      const results: unknown[] = [];
      const tests: Array<(context: unknown) => Promise<void>> = [];
      runInNewContext(result.content.replace(/^import[^\n]+\n/, ''), {
        results,
        test: 'original test',
        bench: 42,
        _test: (_name: string, fn: (context: unknown) => Promise<void>) => tests.push(fn),
      });
      expect(results).toEqual([]);
      for (const fn of tests) {
        await fn({ bench: (_name: string, workload: () => void) => ({ run: workload }) });
      }
      expect(results).toEqual([['original test', 42], 'plain']);
      expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
    },
  );

  it('preserves comments and trailing commas', () => {
    const result = migrate(`import { bench } from 'vitest';
bench('comments', /* workload */ () => { /* inside */ work(); }, /* trailing */);`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('/* workload */');
    expect(result.content).toContain('/* inside */');
    expect(result.content).toContain('/* trailing */');
    expect(migrate(result.content).content).toBe(result.content);
  });

  it.each([
    `function workload() { return 42; }`,
    `const workload = () => 42;`,
    `const workload = async function () { return 42; };`,
    `const original = () => 42; const workload = original;`,
  ])('resolves a local zero-argument workload: %s', (declaration) => {
    const result = migrate(
      `import { bench } from 'vitest'; ${declaration} bench('local', workload);`,
    );
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('const _benchFn = workload;');
    expect(result.content).toContain(`await bench('local', _benchFn).run()`);
    expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
  });

  it.each([
    `let workload = () => 1; bench('mutable', workload); workload = () => 2;`,
    `const workload = (task) => task; bench('context', workload);`,
    `function* workload() { yield 1; } bench('generator', workload);`,
    `const a = b; const b = a; bench('cycle', a);`,
    `import { workload } from './workload.js'; bench('import', workload);`,
    `declare function workload(): void; bench('declared', workload);`,
  ])('does not guess callback semantics: %s', (body) => {
    const input = `import { bench } from 'vitest'; ${body}`;
    const result = migrate(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
  });

  it.each([
    `bench(getName(), workload);`,
    `(bench(getName(), workload));`,
    `bench((getName(), 'dynamic'), workload);`,
    `if (true) bench(getName(), workload); else throw Error('unreachable');`,
    `for (let i = 0; i < 1; i++) bench(getName(), workload);`,
    `label: bench(getName(), workload);`,
  ])(
    'captures dynamic names once at registration and preserves callback identity: %s',
    async (call) => {
      const result = migrate(
        `function workload() { events.push('work'); }
      ${call}
      events.push('registered');`,
        true,
      );
      expect(result.findings).toEqual([]);
      const events: string[] = [];
      const tests: Array<{ name: string; fn: (context: unknown) => Promise<void> }> = [];
      const context = {
        events,
        getName: () => {
          events.push('name');
          return 'dynamic';
        },
        test: (name: string, fn: (context: unknown) => Promise<void>) => tests.push({ name, fn }),
      };
      runInNewContext(result.content, context);
      const original = runInNewContext('workload', context);
      expect(events).toEqual(['name', 'registered']);
      expect(tests.map(({ name }) => name)).toEqual(['dynamic']);
      await tests[0].fn({
        bench: (name: string, fn: () => void) => {
          expect(name).toBe('dynamic');
          expect(fn).toBe(original);
          return {
            run: async () => {
              fn();
              fn();
            },
          };
        },
      });
      expect(events).toEqual(['name', 'registered', 'work', 'work']);
      expect(migrate(result.content, true)).toEqual({ content: result.content, findings: [] });
    },
  );

  it('retains argument evaluation order and temporal dead zones', () => {
    const result = migrate(`bench(getName(), workload); const workload = () => {};`, true);
    expect(result.findings).toEqual([]);
    const events: string[] = [];
    expect(() =>
      runInNewContext(result.content, {
        getName: () => {
          events.push('name');
          return 'case';
        },
        test: () => {
          events.push('test');
        },
      }),
    ).toThrow(/before initialization/);
    expect(events).toEqual(['name']);
  });

  it('captures a changing name for an inline workload and preserves lexical this', async () => {
    const result = migrate(
      `let name = 'before';
      bench(name, () => this.value);
      name = 'after';`,
      true,
    );
    expect(result.findings).toEqual([]);
    const tests: Array<{ name: string; fn: (context: unknown) => Promise<void> }> = [];
    runInNewContext(result.content, {
      value: 42,
      test: (name: string, fn: (context: unknown) => Promise<void>) => tests.push({ name, fn }),
    });
    expect(tests[0].name).toBe('before');
    await tests[0].fn({
      bench: (name: string, fn: () => number) => ({
        run: async () => {
          expect(name).toBe('before');
          expect(fn()).toBe(42);
        },
      }),
    });
  });

  it.each([
    `bench('options', () => work(), { time: 10 });`,
    `bench('callback', workload);`,
    `bench('context', (task) => work(task));`,
    `bench('generator', function* () { yield 1; });`,
    `bench.each(cases)('case', () => work());`,
    `bench.skipIf(condition)('case', () => work());`,
    `const alias = bench; alias('case', () => work());`,
    `register(bench);`,
    `export { bench };`,
    `const result = bench('value', () => work());`,
    `function helper() { bench('helper', () => work()); }`,
    `bench('outer', () => { bench('inner', () => work()); });`,
    `bench?.('optional', () => work());`,
  ])('retains unsupported references without retargeting their import: %s', (call) => {
    const input = `import { bench } from 'vitest'; ${call}`;
    const result = migrate(input);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
  });

  it('does not partly migrate an import shared by direct and escaped references', () => {
    const input = `import { bench } from 'vitest'; bench('direct', () => work()); register(bench);`;
    expect(migrate(input).content).toBe(input);
  });

  it('leaves another library and shadowed bench parameters untouched', () => {
    const input = `import { bench } from 'another-library';
bench('other', () => work());
function helper(bench) { bench('local', () => work()); }`;
    expect(migrate(input, true)).toEqual({ content: input, findings: [] });
  });

  it('registers globals without running workloads and awaits benchmark execution', async () => {
    const input = `describe('group', () => {
  const value = 42;
  bench('workload', async () => { await Promise.resolve(); results.push(value); });
});`;
    const result = migrate(input, true);
    expect(result.findings).toEqual([]);
    const results: number[] = [];
    const tests: Array<{ name: string; fn: (context: unknown) => Promise<void> }> = [];
    runInNewContext(result.content, {
      results,
      describe: (_name: string, fn: () => void) => fn(),
      test: (name: string, fn: (context: unknown) => Promise<void>) => tests.push({ name, fn }),
    });
    expect(results).toEqual([]);
    expect(tests.map(({ name }) => name)).toEqual(['workload']);
    await tests[0].fn({
      bench: (name: string, fn: () => Promise<void>) => ({
        run: async () => {
          expect(name).toBe('workload');
          await fn();
          await fn();
        },
      }),
    });
    expect(results).toEqual([42, 42]);
  });

  it.each([
    'vitest bench',
    'vitest bench --run',
    'vp test bench',
    'pnpm exec vitest bench',
    'vitest bench && echo done',
  ])('keeps the supported command %s', (command) => {
    expect(migrateVitestV5Command('package.json', command, true)).toEqual({
      content: command,
      findings: [],
    });
  });
});

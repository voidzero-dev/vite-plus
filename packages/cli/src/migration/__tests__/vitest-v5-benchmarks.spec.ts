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
      expect(result.content).toContain('import { test as _test, describe }');
      expect(result.content).toContain(
        `_test('parse', async ({ bench: _bench }) => { await _bench('parse', () => { JSON.parse(input); }).run(); });`,
      );
      expect(result.content).toContain(
        `await _bench('async', async () => { await Promise.resolve(input); }).run();`,
      );
      expect(migrate(result.content)).toEqual({ content: result.content, findings: [] });
    },
  );

  it.each(['skip', 'only', 'todo'])('moves bench.%s to the enclosing test', (modifier) => {
    const result = migrate(
      `import { bench } from 'vitest'; bench.${modifier}('case', () => work());`,
    );
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`_test.${modifier}('case', async ({ bench: _bench })`);
    expect(result.content).toContain(`await _bench('case', () => work()).run();`);
  });

  it('migrates a todo with no workload', () => {
    const result = migrate(`import { bench } from 'vitest'; bench.todo('later');`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`_test.todo('later');`);
    expect(result.content).not.toContain('.run()');
  });

  it('migrates namespace imports and preserves an existing fixture', () => {
    const result = migrate(`import * as v from 'vitest';
v.bench('old', () => work());
v.test('new', async ({ bench }) => { await bench('new', () => work()).run(); });`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(`v.test('old', async ({ bench: _bench })`);
    expect(result.content).toContain(
      `v.test('new', async ({ bench }) => { await bench('new', () => work()).run(); });`,
    );
    expect(migrate(result.content).content).toBe(result.content);
  });

  it('avoids collisions with existing test and bench identifiers', () => {
    const result = migrate(`import { bench, test } from 'vitest';
const _test = 1, _bench = 2;
bench('scope', function () { consume(_test, _bench, test); });`);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('test as _test2');
    expect(result.content).toContain('bench: _bench2');
    expect(result.content).toContain('function () { consume(_test, _bench, test); }');
  });

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
    `bench('options', () => work(), { time: 10 });`,
    `bench('callback', workload);`,
    `bench(getName(), () => work());`,
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

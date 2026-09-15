import { runInNewContext } from 'node:vm';

import { describe, expect, it } from 'vitest';

import { migrateVitestV5Command } from '../vitest-v5/commands.ts';
import { migrateVitestV5Config } from '../vitest-v5/config.ts';

const config = (test: string) =>
  migrateVitestV5Config('vitest.config.js', `export default { test: { ${test} } };`, {
    preserveV4: true,
  });

function resolved(content: string): Record<string, unknown> {
  return runInNewContext(content.replace('export default', 'result ='), {}).test;
}

describe('Vitest v5 benchmark output config migration', () => {
  it.each(['outputFile', 'outputJson'])(
    'moves benchmark.%s to the JSON reporter without enabling stdout',
    (key) => {
      const result = config(`benchmark: { ${key}: 'reports/bench.json', include: ['*.bench.js'] }`);
      expect(resolved(result.content)).toEqual({
        clearMocks: false,
        reporters: ['default', 'json'],
        outputFile: { json: 'reports/bench.json' },
        benchmark: { include: ['*.bench.js'] },
      });
      expect(result.findings).toEqual([
        expect.objectContaining({ code: 'benchmark-output', severity: 'review' }),
      ]);
      expect(
        migrateVitestV5Config('vitest.config.js', result.content, { preserveV4: true }),
      ).toEqual({ content: result.content, findings: [] });
    },
  );

  it.each(['default', 'verbose'])(
    'moves the built-in %s reporter without duplicate output',
    (name) => {
      const result = config(`benchmark: { reporters: '${name}' }`);
      expect(resolved(result.content)).toEqual({
        clearMocks: false,
        reporters: [name],
        benchmark: {},
      });
      expect(result.findings).toEqual([]);
    },
  );

  it('retains explicit reporters and unrelated output destinations', () => {
    const result = config(`reporters: ['verbose', 'junit'],
      outputFile: { junit: 'tests.xml' },
      benchmark: { outputJson: 'bench.json' }`);
    expect(resolved(result.content)).toEqual({
      clearMocks: false,
      reporters: ['verbose', 'junit', 'json'],
      outputFile: { junit: 'tests.xml', json: 'bench.json' },
      benchmark: {},
    });
  });

  it.each([`'bench.json'`, `{ json: 'bench.json' }`])(
    'reuses an equivalent destination: %s',
    (output) => {
      const result = config(`reporters: ['default', 'json'], outputFile: ${output},
      benchmark: { reporters: ['default'], outputFile: 'bench.json', outputJson: 'bench.json' }`);
      expect(resolved(result.content).reporters).toEqual(['default', 'json']);
      expect(resolved(result.content).benchmark).toEqual({});
      expect(result.content).not.toContain('stdout');
      expect(result.findings).toHaveLength(1);
    },
  );

  it('keeps comments around removed properties', () => {
    const result = config(`benchmark: { /* before */ reporters: 'default',
      /* between */ outputJson: 'bench.json', /* after */ include: ['*.bench.js'] }`);
    for (const comment of ['/* before */', '/* between */', '/* after */']) {
      expect(result.content).toContain(comment);
    }
    expect(resolved(result.content).benchmark).toEqual({ include: ['*.bench.js'] });
    expect(result.findings.some(({ severity }) => severity === 'block')).toBe(false);
  });

  it.each([
    `benchmark: { reporters: './custom.js' }`,
    `benchmark: { reporters: 'json' }`,
    `benchmark: { reporters: [] }`,
    `benchmark: { reporters: [['default', {}]] }`,
    `benchmark: { outputJson: destination }`,
    `benchmark: { outputFile: { default: 'bench.json' } }`,
    `benchmark: { outputFile: 'one.json', outputJson: 'two.json' }`,
    `reporters: ['default'], benchmark: { reporters: 'verbose' }`,
    `reporters: ['json'], benchmark: { outputJson: 'bench.json' }`,
    `reporters: [['json', { stdout: true }]], benchmark: { outputJson: 'bench.json' }`,
    `reporters: ['default', /* keep */ 'junit'], benchmark: { outputJson: 'bench.json' }`,
    `outputFile: 'tests.json', benchmark: { outputJson: 'bench.json' }`,
    `outputFile: { json: 'tests.json' }, benchmark: { outputJson: 'bench.json' }`,
    `outputFile: output, benchmark: { outputJson: 'bench.json' }`,
  ])('retains ambiguous or conflicting output settings: %s', (input) => {
    const result = config(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
    expect(result.findings.some(({ code }) => code === 'benchmark-output')).toBe(false);
    expect(result.content).toContain('benchmark: {');
  });

  it('does not move a project reporter into a runner-wide setting', () => {
    const result = config(`projects: [{ test: { benchmark: { outputJson: 'bench.json' } } }]`);
    expect(result.content).toContain("outputJson: 'bench.json'");
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
  });

  it('keeps output settings in an exported defineProject blocked', () => {
    const result = migrateVitestV5Config(
      'project.config.ts',
      `import { defineProject } from 'vitest/config';
      export default defineProject({ test: { benchmark: { outputJson: 'bench.json' } } });`,
      { preserveV4: true },
    );
    expect(result.content).toContain("outputJson: 'bench.json'");
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
  });

  it('does not move output from a merged config fragment', () => {
    const input = `export default { test: { benchmark: { outputJson: 'bench.json' } } };`;
    const result = migrateVitestV5Config('vitest.config.js', input, { preserveV4: true }, true);
    expect(result.content).toContain("outputJson: 'bench.json'");
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
  });

  it('still blocks baseline comparisons after planning a supported output move', () => {
    const result = config(`benchmark: { compare: 'baseline.json', outputJson: 'new.json' }`);
    expect(result.findings).toContainEqual(
      expect.objectContaining({
        code: 'benchmark-api',
        severity: 'block',
        message: expect.stringContaining('benchmark.compare'),
      }),
    );
  });
});

describe('Vitest v5 benchmark JSON command migration', () => {
  it.each(['vitest', 'vp test', 'pnpm exec vitest', 'npm exec -- vitest', 'npx vitest'])(
    'moves literal JSON output in %s',
    (runner) => {
      const result = migrateVitestV5Command(
        'package.json',
        `${runner} bench --run --outputJson=bench.json`,
        true,
      );
      expect(result.content).toBe(
        `${runner} bench --run --reporter=default --reporter=json --outputFile=bench.json`,
      );
      expect(result.findings).toEqual([
        expect.objectContaining({ code: 'benchmark-output', severity: 'review' }),
      ]);
      expect(migrateVitestV5Command('package.json', result.content, true)).toEqual({
        content: result.content,
        findings: [],
      });
    },
  );

  it.each([
    [
      `vitest bench --reporters verbose --outputJson=bench.json`,
      `vitest bench --reporters verbose --reporter=json --outputFile=bench.json`,
    ],
    [
      `vitest bench --outputJson 'reports/bench results.json' --run`,
      `vitest bench --reporter=default --reporter=json --outputFile='reports/bench results.json' --run`,
    ],
    [
      `vitest bench --outputJson="bench results.json" --reporter verbose`,
      `vitest bench --reporter=json --outputFile="bench results.json" --reporter verbose`,
    ],
    [
      `vitest bench --reporter=json --outputJson=bench.json`,
      `vitest bench --reporter=json --outputFile=bench.json`,
    ],
    [
      `vitest bench --outputFile=bench.json --outputJson bench.json`,
      `vitest bench --outputFile=bench.json --reporter=default --reporter=json`,
    ],
  ])('preserves quoting and explicit destinations in %s', (input, output) => {
    const result = migrateVitestV5Command('package.json', input, true);
    expect(result.content).toBe(output);
    expect(result.findings.map(({ code }) => code)).toEqual(['benchmark-output']);
  });

  it.each([
    'vitest bench --outputJson=$OUTPUT',
    'vitest bench --outputJson *.json',
    'vitest bench --outputJson=bench.json && echo done',
    'vitest bench --outputJson=one.json --outputJson=two.json',
    'vitest bench --outputJson=bench.json --outputFile=tests.json',
    'vitest bench --outputJson=bench.json --outputFile.json=tests.json',
    'vitest bench --outputJson',
    'vitest bench --outputJson --run',
    'vitest bench --outputJson=bench.json --reporter',
    'vitest bench --outputJson=bench.json --reporter=junit',
    'vitest bench --outputJson=bench.json --reporter=./custom.js',
    'vitest bench --outputJson=bench.json --reporters=junit',
    'vitest bench --testNamePattern "--outputJson=bench.json"',
    'vitest bench -t "--outputJson=bench.json"',
    'vitest bench --compare=baseline.json',
  ])('keeps unresolved output or comparison flags blocked: %s', (input) => {
    const result = migrateVitestV5Command('package.json', input, true);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
    );
  });

  it('does not interpret file filters after -- as removed flags', () => {
    const input = 'vitest bench -- --outputJson=example --compare';
    expect(migrateVitestV5Command('package.json', input, true)).toEqual({
      content: input,
      findings: [],
    });
  });
});

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { parse } from '@babel/parser';
import { afterEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  applyVitestV5Migration,
  createVitestV5CompatibilityConfig,
  finishVitestV5Migration,
  formatVitestV5Findings,
  planVitestV5Migration,
} from '../migrator.ts';
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
  parse(result.content, { sourceType: 'module', plugins: ['typescript'] });
  return result;
}
function source(input: string, options = v4) {
  const result = migrateVitestV5Source('example.test.ts', input, options);
  parse(result.content, { sourceType: 'module', plugins: ['typescript'] });
  return result;
}

describe('Vitest v5 config compatibility', () => {
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
    expect(plan.changes.some(({ file }) => file.endsWith('/node.test.ts'))).toBe(false);
    expect(plan.changes.find(({ file }) => file.endsWith('/browser.test.ts'))?.after).toContain(
      "expect(element).toMatchTextContent('partial')",
    );
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
      expect(plan.changes.some(({ file }) => file.endsWith('/shared.test.ts'))).toBe(false);
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
    expect(plan.changes.find(({ file }) => file.endsWith('/browser.test.ts'))?.after).toContain(
      "expect.element(element).toMatchTextContent('partial')",
    );
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
    expect(plan.changes.some(({ file }) => file.endsWith('/node.test.ts'))).toBe(false);
    expect(plan.changes.find(({ file }) => file.endsWith('/browser.test.ts'))?.after).toContain(
      'toMatchTextContent',
    );
  });

  it('does not infer helper-module ownership from default test globs', () => {
    const root = project({
      'vite.config.ts': 'export default { test: { browser: { enabled: true } } };',
      'assertions.ts': `import { expect } from 'vitest';\nexport function check(element) { expect(element).toHaveTextContent('partial'); }`,
    });
    const plan = planProject(root);
    expect(plan.changes.some(({ file }) => file.endsWith('/assertions.ts'))).toBe(false);
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
    expect(plan.changes.some(({ file }) => file.endsWith('/shared.test.ts'))).toBe(false);
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

  it.each(['reporters', 'outputFile', 'compare', 'outputJson'])(
    'blocks the removed benchmark.%s option',
    (key) => {
      const result = config(`export default { test: { benchmark: { ${key}: 'old' } } };`);
      expect(result.findings).toEqual([
        expect.objectContaining({ code: 'benchmark-api', severity: 'block' }),
      ]);
      expect(result.findings[0].message).toContain(`benchmark.${key}`);
    },
  );

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

  it('blocks unsupported active internals and benchmarks', () => {
    const result = source(`import { startTests } from '@vitest/runner';
import * as internals from 'vitest/internal/module-runner';
import { bench } from 'vitest';
bench('old', () => {});`);
    expect(result.findings.filter((finding) => finding.severity === 'block')).toHaveLength(3);
  });

  it('blocks global benchmarks but preserves the new test-context fixture', () => {
    const result = migrateVitestV5Source(
      'example.test.ts',
      `bench('old', () => {});
test('new', ({ bench }) => { bench('new', () => {}); });`,
      { ...v4, globals: true },
    );
    expect(result.findings.filter((finding) => finding.code === 'benchmark-api')).toEqual([
      expect.objectContaining({ severity: 'block', line: 1 }),
    ]);
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
    expect(plan.changes.some(({ file }) => file.endsWith('/tsconfig.json'))).toBe(false);
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
    const state = JSON.parse(
      fs.readFileSync(path.join(root, '.vite-plus/migrations.json'), 'utf8'),
    );
    expect(state.vitest5['packages/unit'].configless).toBe(false);
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

  it('does not rewrite new v5 assertions in a previously migrated configless project', () => {
    const root = project();
    const plan = planProject(root);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
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
    ).toEqual(['node-runtime', 'removed-api']);
    expect(plan.changes.length).toBeGreaterThan(0);
    expect(() => applyVitestV5Migration(plan)).toThrow('blocking');
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toBe(
      'export default { test: {} };',
    );
    expect(fs.existsSync(path.join(root, '.vite-plus'))).toBe(false);
  });

  it('checks nvmrc, runtime metadata, public engines, CI matrices, and container images', () => {
    const root = project({
      '.nvmrc': '25',
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        engines: { node: '>=18' },
        devEngines: { runtime: { name: 'node', version: '22.12.0' } },
      }),
      '.github/workflows/test.yml':
        'jobs:\n  test:\n    strategy:\n      matrix:\n        node: [20, 22, 24, 25, 26]\n',
      Dockerfile: 'FROM node:20-alpine\n',
    });
    const findings = planProject(root).findings.filter(
      (finding) => finding.code === 'node-runtime',
    );
    expect(findings.filter((finding) => finding.severity === 'block')).toHaveLength(5);
    expect(findings).toContainEqual(
      expect.objectContaining({
        severity: 'review',
        message: expect.stringContaining('public engine contract'),
      }),
    );
  });

  it('does not treat source properties, comments, or Dev Container feature tags as Node images', () => {
    const root = project({
      'worker.js': 'export const options = { node:2 };',
      Dockerfile: '# FROM node:20\nFROM custom/node:1\nRUN echo node:2\n',
      'compose.yml': 'services:\n  web:\n    image: custom/node:1\n    command: echo node:2\n',
      '.devcontainer/devcontainer.json': `{
        // The feature tag is not the Node runtime version.
        "features": { "ghcr.io/devcontainers/features/node:1": { "version": "lts" } }
      }`,
    });
    const findings = planProject(root).findings.filter(({ code }) => code === 'node-runtime');
    expect(findings).toEqual([
      expect.objectContaining({
        severity: 'review',
        message: expect.stringContaining('Dev Container Node feature (lts)'),
      }),
    ]);
  });

  it.each([
    ['Dockerfile', 'FROM --platform=linux/amd64 node:20-alpine AS build\n'],
    ['Dockerfile.test', 'FROM docker.io/library/node:25.9.0@sha256:abc\n'],
    ['Containerfile', 'FROM library/node:20\n'],
    ['compose.yaml', 'services:\n  test:\n    image: node:20-alpine\n'],
    ['.github/workflows/test.yml', 'jobs:\n  test:\n    container: node:25\n'],
    [
      '.github/workflows/test.yml',
      'jobs:\n  test:\n    container:\n      image: docker.io/node:20\n',
    ],
    ['.devcontainer/devcontainer.json', '{ "image": "node:20" }'],
    [
      '.devcontainer/devcontainer.json',
      '{ "features": { "ghcr.io/devcontainers/features/node:1": { "version": "20" } } }',
    ],
  ])('checks a Node runtime in %s', (file, content) => {
    const findings = planProject(project({ [file]: content })).findings.filter(
      ({ code }) => code === 'node-runtime',
    );
    expect(findings).toEqual([expect.objectContaining({ severity: 'block' })]);
  });

  it('keeps a library public engine contract separate from its test runtime pin', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        engines: { node: '20.x' },
      }),
      '.node-version': '22.18.0',
    });
    const plan = planProject(root);
    expect(plan.findings).toContainEqual(
      expect.objectContaining({ code: 'node-runtime', severity: 'review' }),
    );
    expect(plan.findings.some(({ severity }) => severity === 'block')).toBe(false);
  });

  it('checks Volta before its pin is migrated, but respects a higher-priority pin file', () => {
    const root = project({
      'package.json': JSON.stringify({
        devDependencies: { vitest: '4.1.11' },
        volta: { node: '20.19.0' },
      }),
    });
    expect(planProject(root).findings).toContainEqual(
      expect.objectContaining({
        code: 'node-runtime',
        severity: 'block',
        message: expect.stringContaining('volta.node'),
      }),
    );
    fs.writeFileSync(path.join(root, '.nvmrc'), '24.11.0');
    expect(planProject(root).findings.some(({ severity }) => severity === 'block')).toBe(false);
  });

  it('keeps configless projects configless and records completion', () => {
    const root = project({ '.gitignore': '.vitest-reports/\n__screenshots__/\n' });
    const plan = planProject(root);
    expect(plan.findings.some((finding) => finding.code === 'configless-defaults')).toBe(true);
    applyVitestV5Migration(plan);
    finishVitestV5Migration(plan);
    expect(fs.existsSync(path.join(root, 'vite.config.ts'))).toBe(false);
    expect(fs.readFileSync(path.join(root, '.gitignore'), 'utf8')).toBe(
      '.vitest-reports/\n__screenshots__/\n.vitest/\n',
    );
    expect(
      JSON.parse(fs.readFileSync(path.join(root, '.vite-plus/migrations.json'), 'utf8')).vitest5[
        '.'
      ].configless,
    ).toBe(true);
    expect(
      planProject(root).findings.some((finding) => finding.code === 'configless-defaults'),
    ).toBe(true);
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
    applyVitestV5Migration(next);
    finishVitestV5Migration(next);
    expect(fs.readFileSync(path.join(root, 'vite.config.ts'), 'utf8')).toBe(
      'export default { test: {} };',
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
    expect(
      planProject(root).findings.some((finding) => finding.code === 'nested-hoisted-mock'),
    ).toBe(true);
    const report = formatVitestV5Findings(plan);
    expect(report).toContain('Vitest v5:');
    expect(report).toContain('mock.test.ts\n  1:');
  });

  it('retains deferred resolveConfig reviews without reapplying v4 source edits', () => {
    const input = `import { resolveConfig } from 'vitest/node';
const pair = await resolveConfig();
export const config = pair.viteConfig;`;
    const root = project({ 'runner.ts': input });
    const plan = planProject(root);
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'resolve-config' }));
    applyVitestV5Migration(plan);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'resolve-config' }),
    );
    const repeated = planProject(root);
    expect(repeated.findings).toContainEqual(expect.objectContaining({ code: 'resolve-config' }));
    expect(repeated.changes).toEqual([]);
    expect(fs.readFileSync(path.join(root, 'runner.ts'), 'utf8')).toBe(input);
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
    expect(planProject(root).findings.some(({ code }) => code === 'resolve-config')).toBe(false);
  });

  it('uses portable workspace keys in the committed migration state', () => {
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
    const state = JSON.parse(
      fs.readFileSync(path.join(root, '.vite-plus/migrations.json'), 'utf8'),
    );
    expect(Object.keys(state.vitest5).toSorted()).toEqual(['.', 'packages/a']);
    expect(
      planVitestV5Migration(workspace).projects.every(({ options }) => !options.preserveV4),
    ).toBe(true);
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

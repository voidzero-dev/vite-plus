import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  applyVitestV5Migration,
  finishVitestV5Migration,
  planVitestV5Migration,
} from '../migrator.ts';
import { parseSource } from '../vitest-v5/ast.ts';
import { migrateVitestV5Config } from '../vitest-v5/config.ts';
import { migrateVitestV5Source } from '../vitest-v5/source.ts';

const directories: string[] = [];
afterEach(() => {
  for (const directory of directories.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

function workspace(files: Record<string, string>) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-config-scopes-'));
  directories.push(root);
  for (const [name, content] of Object.entries({
    'package.json': '{"private":true,"devDependencies":{"vitest":"4.1.11"}}',
    'pnpm-workspace.yaml': 'packages:\n  - web\n  - browser\n',
    'web/package.json':
      '{"name":"web","devDependencies":{"vitest":"4.1.11"},"scripts":{"test":"vitest"}}',
    'browser/package.json':
      '{"name":"browser","devDependencies":{"vitest":"4.1.11"},"scripts":{"test":"vitest"}}',
    ...files,
  })) {
    const file = path.join(root, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, content);
  }
  const plan = planVitestV5Migration({
    rootDir: root,
    packageManager: PackageManager.pnpm,
    packages: ['web', 'browser'].map((name) => ({ name, path: name })),
  });
  return { root, plan, read: (file: string) => fs.readFileSync(path.join(root, file), 'utf8') };
}

describe('Vitest config factory ownership', () => {
  it.each([
    `export default ({ mode }) => ({ test: { globals: true } });`,
    `export default function ({ mode }) { const isTest = mode === 'test'; return { plugins: isTest ? [] : plugins(), test: { globals: true } }; }`,
    `const config = defineConfig(async ({ mode }) => { const isTest = mode === 'test'; return { ...(!isTest ? { server: { port: 3000 }, ssr: {} } : {}), test: { globals: true } }; }); export default config;`,
  ])('migrates a single static returned config: %s', (declaration) => {
    const input = `import { defineConfig } from 'vitest/config'; ${declaration}`;
    const result = migrateVitestV5Config('vite.config.ts', input, { preserveV4: true });
    expect(result.findings).toEqual([]);
    expect(result.content.match(/clearMocks: false/g)).toHaveLength(1);
    parseSource('vite.config.ts', result.content);
    expect(
      migrateVitestV5Config('vite.config.ts', result.content, { preserveV4: true }).content,
    ).toBe(result.content);
    expect(migrateVitestV5Config('vite.config.ts', input, { preserveV4: false })).toEqual({
      content: input,
      findings: [],
    });
  });

  it.each([
    `() => { if (condition) return other; return { test: { globals: true } }; }`,
    `() => { const config = { test: { globals: true } }; mutate(config); return config; }`,
    `() => { const config = { test: { globals: true } }; const result = mutate(config); return config; }`,
    `() => ({ ...unknown, test: { globals: true } })`,
    `() => ({ ...(condition ? { root: './elsewhere' } : {}), test: { globals: true } })`,
    `() => ({ test: { globals: true }, ...(condition ? { test: other } : {}) })`,
  ])('retains review for unresolved factories and spreads: %s', (factory) => {
    const input = `import { defineConfig } from 'vitest/config'; export default defineConfig(${factory});`;
    const { plan } = workspace({
      'web/vite.config.ts': input,
      'web/example.test.ts': `test.sequential('works', () => {});`,
    });
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'dynamic-config' }));
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
    expect(plan.changes.some(({ file }) => file.endsWith('example.test.ts'))).toBe(false);
    expect(migrateVitestV5Config('vite.config.ts', input, { preserveV4: true }).content).toBe(
      input,
    );
  });

  it.each(['', 'test: { clearMocks: false },'])(
    'keeps a tooling root separate from child Node/browser projects (%s)',
    (defaults) => {
      const nodeSource = `import { expect, vi } from 'vitest'; vi.mock('./dependency'); expect(element).toHaveTextContent('partial'); test.sequential('works', () => {});`;
      const { plan, read } = workspace({
        'vite.config.ts': `export default { ${defaults} fmt: { semi: false } };`,
        'web/vite.config.ts': `import { defineConfig } from 'vite-plus'; export default defineConfig(({ mode }) => {
        const isTest = mode === 'test';
        return { plugins: isTest ? [] : plugins(), ...(!isTest ? { server: {}, optimizeDeps: {} } : {}),
          test: { environment: 'happy-dom', globals: true, setupFiles: ['./setup.ts'] } };
      });`,
        'web/example.test.ts': nodeSource,
        'web/setup.ts': 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });',
        'browser/vitest.config.ts': `export default { test: { globals: true, browser: { enabled: true } } };`,
        'browser/example.test.ts': `expect(element).toHaveTextContent('partial');`,
        'web/public/editor.js': `export const result = collector.collect(options);`,
      });
      expect(plan.findings).toEqual([]);
      applyVitestV5Migration(plan);
      expect(read('web/example.test.ts')).toContain('concurrent: false');
      expect(read('web/example.test.ts')).toContain("toHaveTextContent('partial')");
      expect(read('browser/example.test.ts')).toContain("toMatchTextContent('partial')");
      expect(read('web/setup.ts')).toContain('beforeEach(async () => { await expect');
      expect(read('web/vite.config.ts')).toContain('clearMocks: false');
      expect(read('web/public/editor.js')).toBe(
        'export const result = collector.collect(options);',
      );
      expect(finishVitestV5Migration(plan)).toEqual([]);
    },
  );

  it('keeps explicit parent test invocations in the ownership intersection', () => {
    const { plan } = workspace({
      'package.json': '{"devDependencies":{"vitest":"4.1.11"},"scripts":{"test":"vitest"}}',
      'vite.config.ts': `export default { fmt: { semi: false } };`,
      'web/vite.config.ts': `export default { test: { globals: true } };`,
      'web/example.test.ts': `test.sequential('works', () => {});`,
    });
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
    expect(plan.changes.some(({ file }) => file.endsWith('example.test.ts'))).toBe(false);
  });

  it('does not classify a root test environment as tooling-only', () => {
    const { plan } = workspace({
      'vite.config.ts': `export default { test: { environment: 'happy-dom' } };`,
      'web/vite.config.ts': `export default { test: { globals: true } };`,
      'web/example.test.ts': `test.sequential('works', () => {});`,
    });
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
    expect(plan.changes.some(({ file }) => file.endsWith('example.test.ts'))).toBe(false);
  });

  it.each(['{ test: {} }', 'defineConfig(() => ({ test: {} }))'])(
    'does not add defaults through a config alias passed to user code: %s',
    (initializer) => {
      const input = `import { defineConfig } from 'vitest/config'; const config = ${initializer}; mutate(config); export default config;`;
      const result = migrateVitestV5Config('vite.config.ts', input, { preserveV4: true });
      expect(result.content).toBe(input);
      expect(result.findings).toContainEqual(expect.objectContaining({ code: 'dynamic-config' }));
    },
  );

  it('discovers projects referenced by a callback config', () => {
    const { plan, read } = workspace({
      'web/vite.config.ts': `export default () => ({ test: { projects: ['./unit.mjs'] } });`,
      'web/unit.mjs': `export default { test: { globals: true } };`,
      'web/example.test.ts': `test.sequential('works', () => {});`,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/unit.mjs')).toContain('clearMocks: false');
    expect(read('web/example.test.ts')).toContain('concurrent: false');
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it('keeps browser CLI overrides within their owning package', () => {
    const input = `import { expect } from 'vitest'; expect(element).toHaveTextContent('partial');`;
    const { plan } = workspace({
      'web/vite.config.ts': `export default { test: { environment: 'happy-dom' } };`,
      'web/example.test.ts': input,
      'browser/package.json':
        '{"name":"browser","devDependencies":{"vitest":"4.1.11"},"scripts":{"test":"vitest --browser"}}',
      'browser/vitest.config.ts': `export default { test: {} };`,
      'browser/example.test.ts': input,
    });
    expect(plan.findings.filter(({ code }) => code === 'text-content-project')).toEqual([
      expect.objectContaining({ file: expect.stringContaining(`${path.sep}browser${path.sep}`) }),
    ]);
  });

  it('does not diagnose unrelated collect methods, even in a Vitest source file', () => {
    const input = `import { test } from 'vitest'; test('works', () => collector.collect(options));`;
    expect(migrateVitestV5Source('example.test.ts', input, { preserveV4: true })).toEqual({
      content: input,
      findings: [],
    });
  });
});

describe('In-source test ownership', () => {
  const test = `test.sequential('in-source', () => { expect(() => { throw new Error(''); }).toThrow(''); });`;
  const source = `export const add = (a, b) => a + b; if (import.meta.vitest) { ${test} }`;

  it.each([
    [`{ test: { globals: true, includeSource: ['src/*.ts'] } }`, 'src/add.ts', 'src/excluded.ts'],
    [
      `{ root: './app', test: { root: './unit', globals: true, includeSource: ['*.ts'] } }`,
      'unit/add.ts',
      'unit/excluded.ts',
    ],
    [
      `{ root: './app', test: { dir: './unit', globals: true, includeSource: ['*.ts'] } }`,
      'unit/add.ts',
      'unit/excluded.ts',
    ],
    [
      `{ test: { globals: true, includeSource: ['src/*.ts'], projects: [{ extends: true, test: { name: 'unit', includeSource: ['extra/*.ts'] } }] } }`,
      'src/add.ts',
      'src/excluded.ts',
    ],
    [
      `{ test: { projects: [{ test: { root: './unit', globals: true, includeSource: ['*.ts'] } }] } }`,
      'unit/add.ts',
      'unit/excluded.ts',
    ],
  ])('migrates guarded globals and the normal control using %s', (config, selected, excluded) => {
    const { plan, read } = workspace({
      'web/vitest.config.ts': `export default ${config.replace('globals: true,', "globals: true, exclude: ['**/excluded.ts'],")};`,
      [`web/${selected}`]: source,
      [`web/${excluded}`]: source,
      [`web/${path.posix.dirname(selected)}/control.test.ts`]: test,
      [`web/${path.posix.dirname(selected)}/no-guard.ts`]: test,
      'web/unrelated/add.ts': source,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read(`web/${selected}`)).toContain("test('in-source', { concurrent: false }");
    expect(read(`web/${selected}`)).toContain('toThrow(/^$/)');
    expect(read(`web/${selected}`)).toContain(
      'export const add = (a, b) => a + b; if (import.meta.vitest)',
    );
    expect(read(`web/${path.posix.dirname(selected)}/control.test.ts`)).toContain(
      'concurrent: false',
    );
    expect(read(`web/${excluded}`)).toBe(source);
    expect(read(`web/${path.posix.dirname(selected)}/no-guard.ts`)).toBe(test);
    expect(read('web/unrelated/add.ts')).toBe(source);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it.each([true, false])('respects extends: %s when merging includeSource patterns', (inherits) => {
    const { plan, read } = workspace({
      'web/vitest.config.ts': `export default { test: { includeSource: ['src/*.ts'], projects: [{ extends: ${inherits}, test: { globals: true, includeSource: ['extra/*.ts'] } }] } };`,
      'web/src/add.ts': source,
      'web/extra/add.ts': source,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/extra/add.ts')).toContain('concurrent: false');
    if (inherits) {
      expect(read('web/src/add.ts')).toContain('concurrent: false');
    } else {
      expect(read('web/src/add.ts')).toBe(source);
    }
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it('honors a literal CLI discovery directory override', () => {
    const { plan, read } = workspace({
      'web/package.json':
        '{"devDependencies":{"vitest":"4.1.11"},"scripts":{"test":"vitest run --dir ./unit"}}',
      'web/vitest.config.ts': `export default { test: { globals: true, dir: './ignored', includeSource: ['*.ts'] } };`,
      'web/unit/add.ts': source,
      'web/ignored/add.ts': source,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/unit/add.ts')).toContain('concurrent: false');
    expect(read('web/ignored/add.ts')).toBe(source);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it.each(['patterns', "['src/*.ts', ...patterns]"])(
    'reviews unresolved patterns without blocking known test files: %s',
    (patterns) => {
      const { plan, read } = workspace({
        'web/vitest.config.ts': `export default { test: { globals: true, includeSource: ${patterns}, exclude: ['src/excluded.ts'] } };`,
        'web/src/add.ts': source,
        'web/src/excluded.ts': source,
        'web/src/no-guard.ts': test,
        'web/control.test.ts': test,
      });
      expect(plan.findings).toContainEqual(
        expect.objectContaining({
          code: 'global-api-ownership',
          message: expect.stringContaining('test.includeSource'),
        }),
      );
      expect(plan.findings).toContainEqual(
        expect.objectContaining({
          file: expect.stringContaining(`${path.sep}src${path.sep}add.ts`),
          code: 'global-api-ownership',
        }),
      );
      expect(
        plan.findings.some(
          ({ file }) => file.endsWith('excluded.ts') || file.endsWith('no-guard.ts'),
        ),
      ).toBe(false);
      applyVitestV5Migration(plan);
      expect(read('web/src/add.ts')).toBe(source);
      expect(read('web/src/excluded.ts')).toBe(source);
      expect(read('web/src/no-guard.ts')).toBe(test);
      expect(read('web/control.test.ts')).toContain('concurrent: false');
      expect(finishVitestV5Migration(plan)).toContainEqual(
        expect.objectContaining({ code: 'global-api-ownership' }),
      );
    },
  );

  it('does not lose unresolved inherited includeSource patterns', () => {
    const { plan } = workspace({
      'web/vitest.config.ts': `export default { test: { includeSource: patterns, projects: [{ extends: true, test: { globals: true, includeSource: ['extra/*.ts'] } }] } };`,
      'web/src/add.ts': source,
    });
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
    expect(plan.changes.some(({ file }) => file.endsWith('add.ts'))).toBe(false);
  });

  it('reviews conflicting global ownership of an in-source file', () => {
    const { plan, read } = workspace({
      'web/vitest.config.ts': `export default { test: { includeSource: ['src/*.ts'], projects: [{ extends: true, test: { globals: true } }, { extends: true, test: { globals: false } }] } };`,
      'web/src/add.ts': source,
    });
    expect(plan.findings).toContainEqual(expect.objectContaining({ code: 'global-api-ownership' }));
    applyVitestV5Migration(plan);
    expect(read('web/src/add.ts')).toBe(source);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'global-api-ownership' }),
    );
  });

  it('keeps explicit Vitest imports independent of the globals setting', () => {
    const { plan, read } = workspace({
      'web/vitest.config.ts': `export default { test: { globals: false, includeSource: ['src/*.ts'] } };`,
      'web/src/add.ts': `import { test, expect } from 'vitest'; ${source}`,
      'web/src/other.ts': source,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/src/add.ts')).toContain('concurrent: false');
    expect(read('web/src/other.ts')).toBe(source);
  });
});

describe('Storybook project ownership', () => {
  const assertion = `import { expect } from 'vitest'; expect(element).toHaveTextContent('partial');`;
  const integration = `{ extends: true, test: { name: 'integration', environment: 'jsdom', include: ['src/**/*.test.tsx'], setupFiles: ['./setup.ts'] } }`;
  const browser = `{ extends: true, plugins: [stories({ configDir: path.join(dirname, '.storybook-test') })], test: { name: 'component-browser', globals: true, browser: { enabled: true }, setupFiles: ['./browser-setup.ts'] } }`;
  const config = `import path from 'node:path';
    import { fileURLToPath } from 'node:url';
    import { storybookTest as stories } from '@storybook/addon-vitest/vitest-plugin';
    const dirname = path.dirname(fileURLToPath(import.meta.url));
    export default { test: { projects: [${integration}, ${browser}] } };`;
  const main = `import type { StorybookConfig } from '@storybook/react-vite';
    const config: StorybookConfig = { stories: ['../src/routes/**/*.stories.tsx'] };
    export default config;`;
  function project(files: Record<string, string> = {}) {
    return workspace({
      'vite.config.ts': `export default { fmt: { semi: false } };`,
      'web/vitest.config.ts': config,
      'web/.storybook-test/main.ts': main,
      'web/setup.ts': `import '@testing-library/jest-dom/vitest';`,
      'web/browser-setup.ts': assertion,
      'web/src/account.test.tsx': `${assertion}\n${assertion.replace("import { expect } from 'vitest'; ", '')}\n${assertion.replace("import { expect } from 'vitest'; ", '')}`,
      'web/src/routes/account.stories.tsx': assertion,
      'web/src/unrelated.js': `expect(element).toHaveTextContent('partial');`,
      ...files,
    });
  }

  it.each([
    `path.join(dirname, '.storybook-test')`,
    `path.resolve(import.meta.dirname, '.storybook-test')`,
    `path.join(__dirname, '.storybook-test')`,
    `'.storybook-test'`,
  ])('keeps jsdom matchers and migrates browser stories with configDir %s', (directory) => {
    const { plan, read } = project({
      'web/vitest.config.ts': config.replace("path.join(dirname, '.storybook-test')", directory),
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
    expect(read('web/src/routes/account.stories.tsx')).toContain('toMatchTextContent');
    expect(read('web/browser-setup.ts')).toContain('toMatchTextContent');
    expect(read('web/src/unrelated.js')).toBe(`expect(element).toHaveTextContent('partial');`);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it.each([
    `['../src/examples/*.example.ts']`,
    `[{ directory: '../src/examples', files: '*.example.ts', titlePrefix: 'Examples' }]`,
  ])('uses custom story patterns for global API ownership: %s', (stories) => {
    const { plan, read } = project({
      'web/.storybook-test/main.ts': `export default { stories: ${stories} };`,
      'web/src/routes/account.stories.tsx': '',
      'web/src/examples/account.example.ts': `expect(element).toHaveTextContent('partial');`,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/src/examples/account.example.ts')).toContain('toMatchTextContent');
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
    expect(read('web/src/unrelated.js')).toBe(`expect(element).toHaveTextContent('partial');`);
    expect(finishVitestV5Migration(plan)).toEqual([]);
  });

  it.each([
    `export default { stories: await loadStories() };`,
    `export default { stories: ['../src/**/*.stories.tsx'], ...extra };`,
    `export default { stories: ['../src/**/*.stories.tsx'], viteFinal(config) { return config; } };`,
    `export default { stories: ['../src/**/*.stories.tsx'], presets: ['./preset.js'] };`,
    `export default { stories: [{ directory: '../src' }] };`,
  ])('retains ownership review for dynamic discovery: %s', (main) => {
    const { plan, read } = project({ 'web/.storybook-test/main.ts': main });
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        code: 'global-api-ownership',
        message: expect.stringContaining('Storybook test ownership is unresolved'),
      }),
    );
    expect(
      plan.findings.filter(
        ({ code, file }) => code === 'text-content-project' && file.endsWith('account.test.tsx'),
      ),
    ).toHaveLength(3);
    applyVitestV5Migration(plan);
    expect(read('web/src/routes/account.stories.tsx')).toBe(assertion);
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'text-content-project' }),
    );
  });

  it.each([
    `condition && stories({ configDir: '.storybook-test' })`,
    `stories({ configDir: getConfigDir() })`,
    `stories(options)`,
  ])('does not execute or guess dynamic plugin options: %s', (plugin) => {
    const { plan } = project({
      'web/vitest.config.ts': config.replace(
        "stories({ configDir: path.join(dirname, '.storybook-test') })",
        plugin,
      ),
    });
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        code: 'text-content-project',
        message: expect.stringContaining('Storybook test ownership is unresolved'),
      }),
    );
    expect(plan.changes.some(({ file }) => file.endsWith('account.test.tsx'))).toBe(false);
  });

  it('retains review for a genuine browser/jsdom overlap', () => {
    const { plan, read } = project({
      'web/.storybook-test/main.ts': `export default { stories: ['../src/**/*.test.tsx'] };`,
      'web/src/routes/account.stories.tsx': '',
    });
    const reviews = plan.findings.filter(({ code }) => code === 'text-content-project');
    expect(reviews).toHaveLength(3);
    expect(reviews[0].message).toContain(
      'Conflicting Node/browser projects: integration, component-browser',
    );
    applyVitestV5Migration(plan);
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
    expect(finishVitestV5Migration(plan)).toContainEqual(
      expect.objectContaining({ code: 'text-content-project' }),
    );
  });

  it('does not recognize an unrelated function by its name', () => {
    const { plan } = project({
      'web/vitest.config.ts': config.replace(
        '@storybook/addon-vitest/vitest-plugin',
        './unrelated-plugin',
      ),
    });
    expect(
      plan.findings.filter(
        ({ code, file }) => code === 'text-content-project' && file.endsWith('account.test.tsx'),
      ),
    ).toHaveLength(3);
    expect(plan.changes.some(({ file }) => file.endsWith('account.test.tsx'))).toBe(false);
  });

  it('resolves the default config directory and a JavaScript main file', () => {
    const { plan, read } = project({
      'web/vitest.config.ts': config.replace(
        "stories({ configDir: path.join(dirname, '.storybook-test') })",
        'stories()',
      ),
      'web/.storybook/main.js': `export default { stories: ['../src/routes/**/*.stories.tsx'] };`,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
    expect(read('web/src/routes/account.stories.tsx')).toContain('toMatchTextContent');
  });

  it('retains review for a shared jsdom/browser setup file', () => {
    const { plan, read } = project({
      'web/vitest.config.ts': config.replace("'./browser-setup.ts'", "'./setup.ts'"),
      'web/setup.ts': assertion,
      'web/browser-setup.ts': '',
    });
    expect(plan.findings.filter(({ code }) => code === 'text-content-project')).toEqual([
      expect.objectContaining({ file: expect.stringContaining('setup.ts') }),
    ]);
    applyVitestV5Migration(plan);
    expect(read('web/setup.ts')).toBe(assertion);
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
  });

  it.each(['test.root', 'test.dir'])('matches exclusions relative to %s', (setting) => {
    const { plan, read } = project({
      'web/vitest.config.ts': config.replace(
        "name: 'component-browser',",
        `name: 'component-browser', ${setting.slice(5)}: './src/routes', exclude: ['excluded.stories.tsx'],`,
      ),
      'web/browser-setup.ts': '',
      'web/src/routes/browser-setup.ts': '',
      'web/src/routes/excluded.stories.tsx': `expect(element).toHaveTextContent('partial');`,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/src/routes/account.stories.tsx')).toContain('toMatchTextContent');
    expect(read('web/src/routes/excluded.stories.tsx')).toBe(
      `expect(element).toHaveTextContent('partial');`,
    );
  });

  it('uses the Storybook root for setup files in a nested config directory', () => {
    const { plan, read } = project({
      'web/vitest.config.ts': config.replace("'.storybook-test'", "'browser/.storybook'"),
      'web/browser/.storybook/main.ts': `export default { stories: ['../../src/routes/**/*.stories.tsx'] };`,
      'web/browser-setup.ts': '',
      'web/browser/browser-setup.ts': assertion,
    });
    expect(plan.findings).toEqual([]);
    applyVitestV5Migration(plan);
    expect(read('web/browser/browser-setup.ts')).toContain('toMatchTextContent');
    expect(read('web/src/routes/account.stories.tsx')).toContain('toMatchTextContent');
    expect(read('web/src/account.test.tsx').match(/toHaveTextContent/g)).toHaveLength(3);
  });

  it('retains review when a local preset can alter discovery', () => {
    const { plan } = project({
      'web/.storybook-test/presets.js': `export const stories = () => ['../src/**/*.test.tsx'];`,
    });
    expect(plan.findings).toContainEqual(
      expect.objectContaining({
        code: 'text-content-project',
        message: expect.stringContaining('custom presets'),
      }),
    );
    expect(plan.changes.some(({ file }) => file.endsWith('account.test.tsx'))).toBe(false);
  });
});

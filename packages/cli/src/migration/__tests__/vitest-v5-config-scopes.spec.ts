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

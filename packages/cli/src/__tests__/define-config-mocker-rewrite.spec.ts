import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import { defineConfig, rewriteVitePlusTestSpecifier } from '../define-config.ts';

const REWRITE_PLUGIN_NAME = 'vite-plus:vitest-specifier-rewrite';

function pluginName(p: unknown): string | undefined {
  if (
    p &&
    typeof p === 'object' &&
    'name' in p &&
    typeof (p as { name: unknown }).name === 'string'
  ) {
    return (p as { name: string }).name;
  }
  return undefined;
}

describe('rewriteVitePlusTestSpecifier', () => {
  it('is a no-op when source does not mention vite-plus/test', () => {
    const code = "import { describe } from 'vitest';\nimport * as fs from 'node:fs';\n";
    expect(rewriteVitePlusTestSpecifier(code)).toBe(code);
  });

  it("rewrites `from 'vite-plus/test'` to `from 'vitest'`", () => {
    const input = "import { vi } from 'vite-plus/test';\nvi.mock('./foo');\n";
    const expected = "import { vi } from 'vitest';\nvi.mock('./foo');\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('rewrites the double-quoted form too', () => {
    const input = 'import { vi } from "vite-plus/test";\n';
    const expected = 'import { vi } from "vitest";\n';
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite subpaths such as vite-plus/test/browser', () => {
    const input = "import { context } from 'vite-plus/test/browser';\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it('does NOT rewrite a bare string literal containing vite-plus/test', () => {
    const input = "const x = 'vite-plus/test';\nconsole.log(x);\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it("rewrites dynamic `import('vite-plus/test')`", () => {
    const input = "const mod = await import('vite-plus/test');\n";
    const expected = "const mod = await import('vitest');\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it("rewrites `require('vite-plus/test')` while leaving the subpath form alone", () => {
    const input = [
      "const a = require('vite-plus/test');",
      "const b = require('vite-plus/test/browser');",
      '',
    ].join('\n');
    const expected = [
      "const a = require('vitest');",
      "const b = require('vite-plus/test/browser');",
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('preserves all other imports in the file', () => {
    const input = [
      "import { describe, it, expect } from 'vite-plus/test';",
      "import * as fs from 'node:fs';",
      "import { something } from 'vite-plus/test/browser';",
      '',
    ].join('\n');
    const expected = [
      "import { describe, it, expect } from 'vitest';",
      "import * as fs from 'node:fs';",
      "import { something } from 'vite-plus/test/browser';",
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it("does NOT rewrite `from 'vite-plus/test'` inside a template literal", () => {
    const input = [
      "import { it } from 'vite-plus/test';",
      "const fixture = `import { vi } from 'vite-plus/test'`;",
      'it("snapshots fixture", () => { console.log(fixture); });',
      '',
    ].join('\n');
    const expected = [
      "import { it } from 'vitest';",
      "const fixture = `import { vi } from 'vite-plus/test'`;",
      'it("snapshots fixture", () => { console.log(fixture); });',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite the pattern inside a plain string-literal error message', () => {
    const input = [
      "import { expect, it } from 'vite-plus/test';",
      "it('reports the bad specifier', () => {",
      '  const message = "Cannot resolve \'vite-plus/test\'";',
      "  expect(message).toContain('vite-plus/test');",
      '});',
      '',
    ].join('\n');
    const expected = [
      "import { expect, it } from 'vitest';",
      "it('reports the bad specifier', () => {",
      '  const message = "Cannot resolve \'vite-plus/test\'";',
      "  expect(message).toContain('vite-plus/test');",
      '});',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite the pattern inside a line/block comment or string concat', () => {
    const input = [
      "// Reads 'vite-plus/test' off the import map and rewrites it",
      "/* require('vite-plus/test') is the CJS form */",
      "const composed = 'vite-' + 'plus/test';",
      'const literal = \'require("vite-plus/test")\';',
      'console.log(composed, literal);',
      '',
    ].join('\n');
    // None of these are real imports — output should be byte-identical.
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it("rewrites a real `import { vi } from 'vite-plus/test'` statement", () => {
    const input = ["import { vi } from 'vite-plus/test';", "vi.mock('./foo');", ''].join('\n');
    const expected = ["import { vi } from 'vitest';", "vi.mock('./foo');", ''].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });
});

describe('defineConfig project plugin injection', () => {
  it('injects rewrite plugin at the root plugins array', () => {
    const existing: Plugin = { name: 'user-existing-root-plugin' };
    const result = defineConfig({ plugins: [existing] }) as { plugins: unknown[] };

    expect(pluginName(result.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(result.plugins[1])).toBe('user-existing-root-plugin');
  });

  it('injects rewrite plugin into an inline-object project entry, preserving existing plugins', () => {
    const existing: Plugin = { name: 'user-unit-project-plugin' };
    const result = defineConfig({
      test: {
        projects: [
          {
            plugins: [existing],
            test: { name: 'unit', include: ['test/unit/**/*.spec.ts'], environment: 'node' },
          },
        ],
      },
    }) as { test: { projects: unknown[] } };

    const project = result.test.projects[0] as { plugins: unknown[]; test: { name: string } };
    expect(project.test.name).toBe('unit');
    expect(pluginName(project.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(project.plugins[1])).toBe('user-unit-project-plugin');
    // Sanity: the existing plugin reference is preserved (clone shallow-copies the array).
    expect(project.plugins[1]).toBe(existing);
  });

  it('injects rewrite plugin into the return value of a function-shaped project entry', () => {
    const existing: Plugin = { name: 'user-fn-project-plugin' };
    const projectFn = () => ({
      plugins: [existing],
      test: { name: 'nuxt', environment: 'happy-dom' as const },
    });
    const result = defineConfig({
      test: { projects: [projectFn] },
    }) as { test: { projects: unknown[] } };

    const wrapped = result.test.projects[0];
    expect(typeof wrapped).toBe('function');

    // Vitest passes a `ConfigEnv` to the function; we don't depend on its
    // shape here, the wrapper just forwards it.
    const fakeEnv = { mode: 'test', command: 'serve' as const };
    const resolved = (wrapped as (env: typeof fakeEnv) => { plugins: unknown[] })(fakeEnv);
    expect(pluginName(resolved.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(resolved.plugins[1])).toBe('user-fn-project-plugin');
  });

  it('passes string-glob project entries through unchanged', () => {
    const result = defineConfig({
      test: {
        projects: ['./packages/*', './apps/*'],
      },
    }) as { test: { projects: unknown[] } };

    expect(result.test.projects).toEqual(['./packages/*', './apps/*']);
  });

  it('handles projects with no existing plugins array', () => {
    const result = defineConfig({
      test: {
        projects: [
          {
            test: { name: 'no-plugins', environment: 'node' },
          },
        ],
      },
    }) as { test: { projects: unknown[] } };

    const project = result.test.projects[0] as { plugins: unknown[]; test: { name: string } };
    expect(project.test.name).toBe('no-plugins');
    expect(project.plugins).toHaveLength(1);
    expect(pluginName(project.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
  });
});

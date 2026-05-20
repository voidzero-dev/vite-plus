import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import { defineConfig, rewriteVitePlusTestSpecifier } from '../define-config.ts';

const REWRITE_PLUGIN_NAME = 'vite-plus:vitest-specifier-rewrite';
const RESOLVER_PLUGIN_NAME = 'vite-plus:vitest-resolver';

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

  it('rewrites imports in a JSX/TSX source where the ESM lexer cannot parse', () => {
    const input = [
      "import { describe, it, expect, vi } from 'vite-plus/test';",
      "import { render } from 'vitest-browser-react';",
      "import { Suspense } from 'react';",
      '',
      "vi.mock('./router');",
      "import { Route } from './route';",
      '',
      "describe('App', () => {",
      "  it('renders', () => {",
      '    render(<Suspense fallback={<div>Loading...</div>}><Route /></Suspense>);',
      '  });',
      '});',
      '',
    ].join('\n');
    const expected = [
      "import { describe, it, expect, vi } from 'vitest';",
      "import { render } from 'vitest-browser-react';",
      "import { Suspense } from 'react';",
      '',
      "vi.mock('./router');",
      "import { Route } from './route';",
      '',
      "describe('App', () => {",
      "  it('renders', () => {",
      '    render(<Suspense fallback={<div>Loading...</div>}><Route /></Suspense>);',
      '  });',
      '});',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('rewrites dynamic imports in a JSX/TSX source too', () => {
    const input = [
      'function App() {',
      "  const promise = import('vite-plus/test');",
      '  return <div>{promise.toString()}</div>;',
      '}',
      '',
    ].join('\n');
    const expected = [
      'function App() {',
      "  const promise = import('vitest');",
      '  return <div>{promise.toString()}</div>;',
      '}',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite vite-plus/test subpaths in JSX/TSX fallback either', () => {
    const input = [
      "import { describe } from 'vite-plus/test';",
      "import { ctx } from 'vite-plus/test/browser';",
      'function App() { return <div />; }',
      '',
    ].join('\n');
    const expected = [
      "import { describe } from 'vitest';",
      "import { ctx } from 'vite-plus/test/browser';",
      'function App() { return <div />; }',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite the pattern inside a string literal in TSX (oxc-parser fallback)', () => {
    // No real import — there's nothing to rewrite. The substring lives inside
    // a double-quoted string literal, and the file contains JSX which forces
    // the oxc-parser fallback path.
    const input = [
      'function App() {',
      '  const msg = "from \'vite-plus/test\'";',
      '  return <div>{msg}</div>;',
      '}',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it('does NOT rewrite the pattern inside JSX text (oxc-parser fallback)', () => {
    const input = ['function App() {', "  return <p>from 'vite-plus/test'</p>;", '}', ''].join(
      '\n',
    );
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it("rewrites a real import but preserves a string literal containing 'vite-plus/test' in TSX", () => {
    const input = [
      "import { vi } from 'vite-plus/test';",
      'function App() {',
      '  const fixture = "import { vi } from \'vite-plus/test\'";',
      '  return <p>{fixture}</p>;',
      '}',
      '',
    ].join('\n');
    const expected = [
      "import { vi } from 'vitest';",
      'function App() {',
      '  const fixture = "import { vi } from \'vite-plus/test\'";',
      '  return <p>{fixture}</p>;',
      '}',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('rewrites a real dynamic import but preserves a string literal in TSX', () => {
    const input = [
      'function App() {',
      "  const mod = import('vite-plus/test');",
      '  const fixture = "import(\'vite-plus/test\')";',
      '  return <p>{fixture}</p>;',
      '}',
      '',
    ].join('\n');
    const expected = [
      'function App() {',
      "  const mod = import('vitest');",
      '  const fixture = "import(\'vite-plus/test\')";',
      '  return <p>{fixture}</p>;',
      '}',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it("rewrites `export * from 'vite-plus/test'` in TSX (oxc-parser fallback)", () => {
    const input = [
      "export * from 'vite-plus/test';",
      'function App() { return <div />; }',
      '',
    ].join('\n');
    const expected = ["export * from 'vitest';", 'function App() { return <div />; }', ''].join(
      '\n',
    );
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it("rewrites `export { vi } from 'vite-plus/test'` in TSX (oxc-parser fallback)", () => {
    const input = [
      "export { vi } from 'vite-plus/test';",
      'function App() { return <div />; }',
      '',
    ].join('\n');
    const expected = [
      "export { vi } from 'vitest';",
      'function App() { return <div />; }',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('rewrites a real re-export but preserves a string literal containing the same text in TSX', () => {
    const input = [
      "export * from 'vite-plus/test';",
      'function App() {',
      '  const fixture = "export * from \'vite-plus/test\'";',
      '  return <p>{fixture}</p>;',
      '}',
      '',
    ].join('\n');
    const expected = [
      "export * from 'vitest';",
      'function App() {',
      '  const fixture = "export * from \'vite-plus/test\'";',
      '  return <p>{fixture}</p>;',
      '}',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite a local `export { vi }` (no `from` clause) in TSX', () => {
    const input = [
      'const vi = 1;',
      'export { vi };',
      "const note = 'vite-plus/test';",
      'function App() { return <div>{note}</div>; }',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });
});

describe('defineConfig project plugin injection', () => {
  it('injects rewrite + resolver plugins at the root plugins array', () => {
    const existing: Plugin = { name: 'user-existing-root-plugin' };
    const result = defineConfig({ plugins: [existing] }) as { plugins: unknown[] };

    expect(pluginName(result.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(result.plugins[1])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(result.plugins[2])).toBe('user-existing-root-plugin');
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
    expect(pluginName(project.plugins[1])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(project.plugins[2])).toBe('user-unit-project-plugin');
    // Sanity: the existing plugin reference is preserved (clone shallow-copies the array).
    expect(project.plugins[2]).toBe(existing);
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
    expect(pluginName(resolved.plugins[1])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(resolved.plugins[2])).toBe('user-fn-project-plugin');
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
    expect(project.plugins).toHaveLength(2);
    expect(pluginName(project.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(project.plugins[1])).toBe(RESOLVER_PLUGIN_NAME);
  });
});

describe('defineConfig auto-inline deps', () => {
  const AUTO_INLINE = ['@testing-library/jest-dom', '@storybook/test', 'jest-extended'];

  it('injects the auto-inline packages when no inline list is set', () => {
    const result = defineConfig({}) as {
      test?: { server?: { deps?: { inline?: unknown } } };
    };
    expect(result.test?.server?.deps?.inline).toEqual(AUTO_INLINE);
  });

  it('merges with an existing user inline array, preserving order and dedupe', () => {
    const result = defineConfig({
      test: { server: { deps: { inline: ['my-pkg', '@testing-library/jest-dom'] } } },
    }) as { test: { server: { deps: { inline: unknown[] } } } };
    expect(result.test.server.deps.inline).toEqual([
      'my-pkg',
      '@testing-library/jest-dom',
      '@storybook/test',
      'jest-extended',
    ]);
  });

  it("does not override `inline: true` (user opted into 'inline everything')", () => {
    const result = defineConfig({
      test: { server: { deps: { inline: true } } },
    }) as { test: { server: { deps: { inline: unknown } } } };
    expect(result.test.server.deps.inline).toBe(true);
  });

  it('treats a regexp entry that matches an auto-inline pkg as already covered', () => {
    const result = defineConfig({
      test: { server: { deps: { inline: [/^@testing-library\//, /^@storybook\//] } } },
    }) as { test: { server: { deps: { inline: unknown[] } } } };
    // Both '@testing-library/jest-dom' and '@storybook/test' match the regexps;
    // only 'jest-extended' should be appended.
    const inline = result.test.server.deps.inline;
    expect(inline).toHaveLength(3);
    expect(inline[0]).toBeInstanceOf(RegExp);
    expect(inline[1]).toBeInstanceOf(RegExp);
    expect(inline[2]).toBe('jest-extended');
  });

  it('injects auto-inline into each test.projects entry', () => {
    const result = defineConfig({
      test: {
        projects: [
          { test: { name: 'unit', environment: 'node' } },
          {
            test: {
              name: 'browser',
              environment: 'jsdom',
              server: { deps: { inline: ['custom'] } },
            },
          },
        ],
      },
    }) as { test: { projects: unknown[] } };

    const [p0, p1] = result.test.projects as Array<{
      test: { name: string; server?: { deps?: { inline?: unknown } } };
    }>;
    expect(p0.test.name).toBe('unit');
    expect(p0.test.server?.deps?.inline).toEqual(AUTO_INLINE);
    expect(p1.test.name).toBe('browser');
    expect(p1.test.server?.deps?.inline).toEqual(['custom', ...AUTO_INLINE]);
  });

  it('does not re-add an entry that already exists', () => {
    const result = defineConfig({
      test: { server: { deps: { inline: AUTO_INLINE.slice() } } },
    }) as { test: { server: { deps: { inline: unknown[] } } } };
    expect(result.test.server.deps.inline).toEqual(AUTO_INLINE);
  });
});

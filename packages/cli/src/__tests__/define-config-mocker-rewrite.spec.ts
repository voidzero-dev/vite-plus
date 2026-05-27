import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import {
  AUTO_INLINE_DEPS,
  computeAutoInlineList,
  defineConfig,
  rewriteVitePlusTestSpecifier,
} from '../define-config.ts';

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

  it("does NOT rewrite `require('vite-plus/test')` inside a template literal", () => {
    // Regression: previously the regex-based CJS pass anchored `require` at
    // a `\s` boundary, so a `\n` + indentation inside a backtick template
    // literal still matched. Snapshot/fixture strings containing example
    // code must stay byte-identical.
    const input = [
      'const fixture = `',
      "  require('vite-plus/test')",
      '`;',
      'console.log(fixture);',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it("does NOT rewrite `require('vite-plus/test')` inside a plain string literal", () => {
    const input = [
      'const literal = "  require(\'vite-plus/test\')";',
      'console.log(literal);',
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it("does NOT rewrite a member call like `.require('vite-plus/test')`", () => {
    // The AST walker only matches CallExpression nodes whose callee is the
    // identifier `require`. `obj.require(...)` is a MemberExpression callee
    // and must be left alone.
    const input = "const m = obj.require('vite-plus/test');\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it('rewrites a real require call alongside a fixture template literal', () => {
    // Mixed scenario: a real CJS require should be rewritten, but the
    // example code embedded in a template literal must stay untouched.
    const input = [
      "const real = require('vite-plus/test');",
      'const fixture = `',
      "  require('vite-plus/test')",
      '`;',
      'console.log(real, fixture);',
      '',
    ].join('\n');
    const expected = [
      "const real = require('vitest');",
      'const fixture = `',
      "  require('vite-plus/test')",
      '`;',
      'console.log(real, fixture);',
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
  it('injects rewrite + resolver + auto-inline plugins at the root plugins array', () => {
    const existing: Plugin = { name: 'user-existing-root-plugin' };
    const result = defineConfig({ plugins: [existing] }) as { plugins: unknown[] };

    expect(pluginName(result.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(result.plugins[1])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(result.plugins[2])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(result.plugins[3])).toBe('user-existing-root-plugin');
  });

  it('injects rewrite + resolver + auto-inline plugins into an inline-object project entry, preserving existing plugins', () => {
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
    expect(pluginName(project.plugins[2])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(project.plugins[3])).toBe('user-unit-project-plugin');
    // Sanity: the existing plugin reference is preserved (clone shallow-copies the array).
    expect(project.plugins[3]).toBe(existing);
  });

  it('injects plugins into the return value of a function-shaped project entry', () => {
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
    expect(pluginName(resolved.plugins[2])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(resolved.plugins[3])).toBe('user-fn-project-plugin');
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
    expect(project.plugins).toHaveLength(3);
    expect(pluginName(project.plugins[0])).toBe(REWRITE_PLUGIN_NAME);
    expect(pluginName(project.plugins[1])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(project.plugins[2])).toBe(AUTO_INLINE_PLUGIN_NAME);
  });
});

const AUTO_INLINE_PLUGIN_NAME = 'vite-plus:auto-inline-matcher-deps';

/** Builds a mock require-factory where only `installedPkgs` resolve. */
function makeRequireFactory(
  installedPkgs: string[],
): (from: string) => { resolve: (id: string) => string } {
  return (_from: string) => ({
    resolve(id: string) {
      if (installedPkgs.includes(id)) {
        return `/mock/node_modules/${id}/index.js`;
      }
      throw new Error(`Cannot find module '${id}'`);
    },
  });
}

/** A mock require-factory where every package resolves. */
const allInstalledFactory = makeRequireFactory([
  '@testing-library/jest-dom',
  '@storybook/test',
  'jest-extended',
]);

/** A mock require-factory where no auto-inline package resolves. */
const noneInstalledFactory = makeRequireFactory([]);

describe('computeAutoInlineList', () => {
  const ALL = [...AUTO_INLINE_DEPS];

  it('inlines all packages when all are installed and no existing list', () => {
    expect(computeAutoInlineList(undefined, '/project', allInstalledFactory)).toEqual(ALL);
  });

  it('inlines only installed packages — absent ones are skipped', () => {
    const only = makeRequireFactory(['@testing-library/jest-dom']);
    expect(computeAutoInlineList(undefined, '/project', only)).toEqual([
      '@testing-library/jest-dom',
    ]);
  });

  it('returns null when no auto-inline package is installed', () => {
    expect(computeAutoInlineList(undefined, '/project', noneInstalledFactory)).toBeNull();
  });

  it('merges with an existing user inline array, preserving order and deduplicating', () => {
    const existing: (string | RegExp)[] = ['my-pkg', '@testing-library/jest-dom'];
    const result = computeAutoInlineList(existing, '/project', allInstalledFactory);
    expect(result).toEqual([
      'my-pkg',
      '@testing-library/jest-dom',
      '@storybook/test',
      'jest-extended',
    ]);
    // Original array must not be mutated.
    expect(existing).toEqual(['my-pkg', '@testing-library/jest-dom']);
  });

  it("returns null when `inline: true` (user opted into 'inline everything')", () => {
    expect(computeAutoInlineList(true, '/project', allInstalledFactory)).toBeNull();
  });

  it('treats a regexp entry that matches an auto-inline pkg as already covered', () => {
    const existing: (string | RegExp)[] = [/^@testing-library\//, /^@storybook\//];
    const result = computeAutoInlineList(existing, '/project', allInstalledFactory);
    // Both '@testing-library/jest-dom' and '@storybook/test' are covered;
    // only 'jest-extended' should be appended.
    expect(result).toHaveLength(3);
    expect(result![0]).toBeInstanceOf(RegExp);
    expect(result![1]).toBeInstanceOf(RegExp);
    expect(result![2]).toBe('jest-extended');
  });

  it('returns null when all auto-inline packages are already in the existing list', () => {
    const existing: (string | RegExp)[] = [...AUTO_INLINE_DEPS];
    expect(computeAutoInlineList(existing, '/project', allInstalledFactory)).toBeNull();
  });

  it('passes the project root to the require factory', () => {
    const capturedFroms: string[] = [];
    const factory = (from: string) => {
      capturedFroms.push(from);
      return { resolve: (_id: string) => `/mock/node_modules/${_id}/index.js` };
    };
    computeAutoInlineList(undefined, '/custom/root', factory);
    expect(capturedFroms).toEqual(['/custom/root/package.json']);
  });
});

describe('defineConfig auto-inline deps plugin registration', () => {
  it('registers the auto-inline plugin in the root plugins array with enforce:pre and configResolved', () => {
    const result = defineConfig({}) as { plugins: unknown[] };
    const plugin = result.plugins.find(
      (p): p is Record<string, unknown> =>
        !!p && typeof p === 'object' && (p as { name?: unknown }).name === AUTO_INLINE_PLUGIN_NAME,
    );
    expect(plugin).toBeDefined();
    expect(plugin?.enforce).toBe('pre');
    expect(typeof plugin?.configResolved).toBe('function');
  });
});

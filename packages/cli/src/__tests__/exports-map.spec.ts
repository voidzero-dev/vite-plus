/**
 * Regression tests for the generated package.json `exports` map.
 *
 * Node.js package-exports conditions are order-sensitive: when resolving
 * `require('vite-plus/test/config')`, Node walks the condition object and
 * picks the first matching key. `default` matches everything, so a wrongly
 * ordered map like `{ types, default, require }` causes CJS consumers to
 * load the ESM shim — the `.cjs` shim becomes unreachable.
 *
 * These tests pin the invariant that any dual-condition entry emits
 * `require` BEFORE `default` and that runtime resolution returns the
 * expected file extension for each consumer.
 */
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import url from 'node:url';

import ts from 'typescript';
import { describe, expect, it } from 'vitest';

const cliPkgDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '../..');
const cliPkgJsonPath = path.join(cliPkgDir, 'package.json');
const requireFromHere = createRequire(import.meta.url);

type ExportConditions = Record<string, unknown>;

function isConditionObject(value: unknown): value is ExportConditions {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

// `default` is a module-shape artifact, not a named export migration cares
// about; only the named bindings need to survive the `vitest/config` rewrite.
function namedValueExports(mod: Record<string, unknown>): string[] {
  return Object.keys(mod).filter((key) => key !== 'default');
}

describe('package.json exports map', () => {
  it('provides the bundled Vitest Vite peer without relying on project dependencies', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    expect(pkg.dependencies.vite).toBe('workspace:@voidzero-dev/vite-plus-core@*');
    expect(pkg.devDependencies.vite).toBeUndefined();
  });

  it('keeps the reviewed Vitest v5 export surface', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    expect(
      Object.keys(pkg.exports)
        .filter((key) => key === './test' || key.startsWith('./test/'))
        .toSorted(),
    ).toMatchSnapshot();
  });

  it('resolves every runtime test export under Node ESM conditions', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    for (const [key, value] of Object.entries(pkg.exports as Record<string, ExportConditions>)) {
      if (!(key === './test' || key.startsWith('./test/')) || (!value.default && !value.import)) {
        continue;
      }
      const resolved = import.meta.resolve(`vite-plus${key.slice(1)}`);
      expect(fs.existsSync(url.fileURLToPath(resolved)), key).toBe(true);
    }
  });

  it('compiles all typed test exports with NodeNext resolution', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    const entries = Object.entries(pkg.exports as Record<string, ExportConditions>).filter(
      ([key, entry]) =>
        (key === './test' || key.startsWith('./test/')) &&
        (entry.types || (isConditionObject(entry.import) && entry.import.types)),
    );
    const source =
      entries
        .map(([key], index) => `import type * as E${index} from 'vite-plus${key.slice(1)}';`)
        .join('\n') +
      `
import { expect } from 'vite-plus/test';
declare module 'vite-plus/test' {
  interface Matchers<R extends void | Promise<void> = void | Promise<void>, T = unknown> {
    toMatchReceived(expected: T): R;
  }
}
const synchronous: void = expect(42).toMatchReceived(42);
const asynchronous: Promise<void> = expect(Promise.resolve(42)).resolves.toMatchReceived(42);
// @ts-expect-error The received type is number, not string.
expect(42).toMatchReceived('42');
// @ts-expect-error An asynchronous matcher does not return void.
const invalidReturn: void = expect(Promise.resolve(42)).resolves.toMatchReceived(42);
`;
    const filename = path.join(cliPkgDir, '__test_exports__.mts');
    const options: ts.CompilerOptions = {
      noEmit: true,
      strict: true,
      skipLibCheck: true,
      types: [],
      module: ts.ModuleKind.NodeNext,
      target: ts.ScriptTarget.ESNext,
    };
    const host = ts.createCompilerHost(options);
    const getSourceFile = host.getSourceFile.bind(host);
    host.getSourceFile = (file, ...args) =>
      file === filename
        ? ts.createSourceFile(file, source, ts.ScriptTarget.ESNext, true)
        : getSourceFile(file, ...args);
    const program = ts.createProgram([filename], options, host);
    const diagnostics = ts.getPreEmitDiagnostics(program);
    expect(diagnostics.map((d) => ts.flattenDiagnosticMessageText(d.messageText, '\n'))).toEqual(
      [],
    );
  });

  it.each([
    ['coverage', 'vitest/node'],
    ['reporters', 'vitest/node'],
    ['environments', 'vitest/runtime'],
    ['snapshot', 'vitest/runtime'],
    ['mocker', '@vitest/mocker'],
  ])('keeps the complete %s compatibility target', (name, upstream) => {
    const source = fs.readFileSync(path.join(cliPkgDir, 'dist/test', `${name}.js`), 'utf8');
    expect(source).toBe(`export * from '${upstream}';\n`);
  });

  it('every dual-condition entry emits `require` before `default`', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf-8'));
    const exports = pkg.exports as Record<string, unknown>;

    const offenders: Array<{ path: string; order: string[] }> = [];

    function walk(subpath: string, value: unknown) {
      if (!isConditionObject(value)) {
        return;
      }
      const keys = Object.keys(value);
      const requireIdx = keys.indexOf('require');
      const defaultIdx = keys.indexOf('default');
      if (requireIdx !== -1 && defaultIdx !== -1 && requireIdx > defaultIdx) {
        offenders.push({ path: subpath, order: keys });
      }
      for (const [k, v] of Object.entries(value)) {
        walk(`${subpath} > ${k}`, v);
      }
    }

    for (const [subpath, value] of Object.entries(exports)) {
      walk(subpath, value);
    }

    expect(offenders, 'entries with require ordered after default').toEqual([]);
  });

  it.each([
    'browser/context',
    'context',
    'plugins/browser-context',
    'browser-preview/context',
    'browser-playwright/context',
    'browser-webdriverio/context',
  ])('routes the %s runtime alias to the v5 browser virtual module', (name) => {
    expect(fs.readFileSync(path.join(cliPkgDir, 'dist/test', `${name}.js`), 'utf8')).toBe(
      "export * from 'vitest/browser';\n",
    );
  });

  it('./test/config has both `require` and `default`, with `require` first', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf-8'));
    const entry = (pkg.exports as Record<string, unknown>)['./test/config'];
    expect(isConditionObject(entry)).toBe(true);
    const keys = Object.keys(entry as ExportConditions);
    expect(keys).toContain('require');
    expect(keys).toContain('default');
    expect(keys.indexOf('require')).toBeLessThan(keys.indexOf('default'));
  });

  it('`require.resolve("vite-plus/test/config")` resolves to the .cjs shim', () => {
    const resolved = requireFromHere.resolve('vite-plus/test/config');
    expect(resolved.endsWith('.cjs'), `resolved to ${resolved}`).toBe(true);
  });

  it('ESM `import.meta.resolve("vite-plus/test/config")` resolves to the .js shim', () => {
    // import.meta.resolve is sync in modern Node (>= 20.6) and respects the
    // `default` (ESM) condition for ESM consumers.
    const resolved = import.meta.resolve('vite-plus/test/config');
    expect(resolved.endsWith('.js'), `resolved to ${resolved}`).toBe(true);
  });

  it('CJS shim at ./test/config delegates to vitest/config via require()', () => {
    const cfg = requireFromHere('vite-plus/test/config') as Record<string, unknown>;
    expect(cfg).toBeTypeOf('object');
    // vitest/config re-exports defineConfig / configDefaults — sanity-check one.
    expect(typeof cfg.defineConfig).toBe('function');
  });

  it('browser-playwright declaration uses NodeNext-compatible relative specifiers', () => {
    // Matches quoted relative module specifiers such as './node' and '../browser.js'.
    const relativeModuleSpecifier = /(['"])(\.\.?\/[^'"]+)\1/g;
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf-8'));
    const entry = (pkg.exports as Record<string, unknown>)['./test/browser-playwright'];
    expect(isConditionObject(entry)).toBe(true);
    const types = (entry as ExportConditions).types;
    expect(types).toBeTypeOf('string');

    const source = fs.readFileSync(path.resolve(cliPkgDir, types as string), 'utf-8');
    const relativeSpecifiers = Array.from(
      source.matchAll(relativeModuleSpecifier),
      (match) => match[2],
    );
    expect(
      relativeSpecifiers.length,
      'declaration should contain relative specifiers',
    ).toBeGreaterThan(0);
    expect(
      relativeSpecifiers.filter((specifier) => !specifier.endsWith('.js')),
      'declaration contains NodeNext-incompatible relative specifiers',
    ).toEqual([]);
  });
});

// Migrated plugins depend on these entry points resolving the upstream APIs.
describe('Oxlint JS-plugin authoring entrypoints', () => {
  it('re-exports the full @oxlint/plugins value surface', async () => {
    const [lintPlugins, oxlintPlugins] = await Promise.all([
      import('vite-plus/lint/plugins'),
      import('@oxlint/plugins'),
    ]);
    const expected = namedValueExports(oxlintPlugins);
    expect(expected.length, 'sanity: @oxlint/plugins should expose value exports').toBeGreaterThan(
      0,
    );
    const missing = expected.filter(
      (key) => !(key in lintPlugins) || (lintPlugins as Record<string, unknown>)[key] === undefined,
    );
    expect(missing, '@oxlint/plugins value exports missing from vite-plus/lint/plugins').toEqual(
      [],
    );
  });

  it('serves the authoring API to CommonJS too', () => {
    // A `.cts` plugin, or a `.ts` one compiled with `module: commonjs`, emits
    // its import as `require()`. `@oxlint/plugins` ships CJS, so the shim does
    // too. `plugins-dev` instead exposes its ESM entry to both module systems.
    const plugins = requireFromHere('vite-plus/lint/plugins') as Record<string, unknown>;
    expect(plugins.defineRule).toBeTypeOf('function');
    expect(plugins.definePlugin).toBeTypeOf('function');
  });

  it('exposes RuleTester from vite-plus/lint/plugins-dev', async () => {
    const ruleTester = await import('vite-plus/lint/plugins-dev');
    expect(ruleTester.RuleTester).toBeTypeOf('function');
  });

  it('serves the same RuleTester to CommonJS', async () => {
    // Static imports in .cts files compile to require() after migration.
    const ruleTester = requireFromHere('vite-plus/lint/plugins-dev') as Record<string, unknown>;
    const upstream = requireFromHere('oxlint/plugins-dev') as Record<string, unknown>;
    const esm = await import('vite-plus/lint/plugins-dev');
    expect(ruleTester.RuleTester).toBeTypeOf('function');
    expect(ruleTester.RuleTester).toBe(upstream.RuleTester);
    expect(ruleTester.RuleTester).toBe(esm.RuleTester);
  });
});

/**
 * Migration rewrites the `vitest/config` specifier to bare `vite-plus` (see the
 * Rust `import_rewriter.rs` rule and the `prefer-vite-plus-imports` oxlint rule
 * in `oxlint-plugin.ts`). After that rewrite a user's
 * `import { x } from 'vitest/config'` (and the `require(...)` form) becomes
 * `from 'vite-plus'`, so the `vite-plus` root MUST stay a superset of
 * `vitest/config`'s runtime exports. The named re-export lists in `index.ts`
 * (ESM) and `index.cts` (CJS) are SEPARATE manual lists, and both deliberately
 * omit `defineConfig` (local `./define-config.ts` wrapper) and `mergeConfig`
 * (`@voidzero-dev/vite-plus-core`), which are supplied by other paths. These
 * guards assert the *aggregate* surface stays complete on BOTH module systems so
 * a future vitest bump that adds a config export can't silently leave migrated
 * imports `undefined` — and can't be fixed in one entry while the other regresses.
 *
 * Note: these cover the runtime (value) surface only. `vitest/config`'s
 * type-only exports flow through a separate `export type { … }` list in
 * `index.ts`; removal-drift there is already caught by the repo typecheck (a
 * re-export of a deleted type fails to compile).
 */
describe('vite-plus root re-exports the full vitest/config surface', () => {
  it('exposes every vitest/config value export on the ESM vite-plus root', async () => {
    const [vitePlus, vitestConfig] = await Promise.all([
      import('vite-plus'),
      import('vitest/config'),
    ]);
    const expected = namedValueExports(vitestConfig);
    expect(expected.length, 'sanity: vitest/config should expose value exports').toBeGreaterThan(0);
    const missing = expected.filter(
      (key) => !(key in vitePlus) || (vitePlus as Record<string, unknown>)[key] === undefined,
    );
    expect(missing, 'vitest/config value exports missing from the ESM vite-plus root').toEqual([]);
  });

  it('exposes every vitest/config value export on the CJS vite-plus root', () => {
    // `require('vitest/config')` -> `require('vite-plus')` resolves the package
    // root's `require` condition (the index.cts build), a separate manual list.
    const vitePlus = requireFromHere('vite-plus') as Record<string, unknown>;
    const vitestConfig = requireFromHere('vitest/config') as Record<string, unknown>;
    const expected = namedValueExports(vitestConfig);
    expect(expected.length, 'sanity: vitest/config should expose value exports').toBeGreaterThan(0);
    const missing = expected.filter((key) => !(key in vitePlus) || vitePlus[key] === undefined);
    expect(missing, 'vitest/config value exports missing from the CJS vite-plus root').toEqual([]);
  });
});

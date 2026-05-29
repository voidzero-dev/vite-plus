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

import { describe, expect, it } from 'vitest';

const cliPkgDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '../..');
const cliPkgJsonPath = path.join(cliPkgDir, 'package.json');
const requireFromHere = createRequire(import.meta.url);

type ExportConditions = Record<string, unknown>;

function isConditionObject(value: unknown): value is ExportConditions {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

describe('package.json exports map', () => {
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
  // `default` is a module-shape artifact, not a named export migration cares
  // about; only the named bindings need to survive the `vitest/config` rewrite.
  function namedValueExports(mod: Record<string, unknown>): string[] {
    return Object.keys(mod).filter((key) => key !== 'default');
  }

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

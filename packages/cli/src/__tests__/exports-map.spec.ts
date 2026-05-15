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

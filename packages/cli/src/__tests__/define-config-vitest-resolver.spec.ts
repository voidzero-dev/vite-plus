import { fileURLToPath } from 'node:url';

import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import { defineConfig, isVitestFamilySpecifier } from '../define-config.ts';

const RESOLVER_PLUGIN_NAME = 'vite-plus:vitest-resolver';

function findPlugin(plugins: unknown, name: string): Record<string, unknown> | undefined {
  if (!Array.isArray(plugins)) {
    return undefined;
  }
  return plugins.find(
    (p): p is Record<string, unknown> =>
      !!p && typeof p === 'object' && (p as { name?: unknown }).name === name,
  );
}

function idOf(resolved: string | { id: string } | null | undefined): string | undefined {
  return typeof resolved === 'string' ? resolved : resolved?.id;
}

describe('isVitestFamilySpecifier', () => {
  it('matches the bare `vitest` specifier', () => {
    expect(isVitestFamilySpecifier('vitest')).toBe(true);
  });

  it('matches `vitest/internal/browser`', () => {
    expect(isVitestFamilySpecifier('vitest/internal/browser')).toBe(true);
  });

  it('matches `vitest/config`', () => {
    expect(isVitestFamilySpecifier('vitest/config')).toBe(true);
  });

  it('matches `@vitest/browser`', () => {
    expect(isVitestFamilySpecifier('@vitest/browser')).toBe(true);
  });

  it('matches `@vitest/browser/context`', () => {
    expect(isVitestFamilySpecifier('@vitest/browser/context')).toBe(true);
  });

  it('matches `@vitest/expect`', () => {
    expect(isVitestFamilySpecifier('@vitest/expect')).toBe(true);
  });

  it('matches a queried subpath (query stripped before matching)', () => {
    expect(isVitestFamilySpecifier('vitest/internal/browser?v=1')).toBe(true);
  });

  it('does NOT match `vitest-foo` (not a subpath of vitest)', () => {
    expect(isVitestFamilySpecifier('vitest-foo')).toBe(false);
  });

  it('does NOT match the bare scope `@vitest` (no trailing slash)', () => {
    expect(isVitestFamilySpecifier('@vitest')).toBe(false);
  });

  it('does NOT match a relative id', () => {
    expect(isVitestFamilySpecifier('./local')).toBe(false);
  });

  it('does NOT match an absolute id', () => {
    expect(isVitestFamilySpecifier('/abs/path/vitest')).toBe(false);
  });

  it('does NOT match a virtual id', () => {
    expect(isVitestFamilySpecifier('\0virtual')).toBe(false);
  });

  it('does NOT match an unrelated bare specifier', () => {
    expect(isVitestFamilySpecifier('react')).toBe(false);
  });
});

describe('vitePlusVitestResolverPlugin', () => {
  it('is injected into the root plugins array as an enforce:pre plugin with resolveId', () => {
    const result = defineConfig({}) as { plugins: unknown[] };
    const plugin = findPlugin(result.plugins, RESOLVER_PLUGIN_NAME);

    expect(plugin).toBeDefined();
    expect(plugin?.name).toBe(RESOLVER_PLUGIN_NAME);
    expect(plugin?.enforce).toBe('pre');
    expect(typeof plugin?.resolveId).toBe('function');
  });

  it('is injected into each `test.projects` entry (before user plugins)', () => {
    const existing: Plugin = { name: 'user-project-plugin' };
    const result = defineConfig({
      test: {
        projects: [
          { test: { name: 'unit', environment: 'node' } },
          { plugins: [existing], test: { name: 'browser', environment: 'jsdom' } },
        ],
      },
    }) as { test: { projects: unknown[] } };

    for (const project of result.test.projects) {
      const plugins = (project as { plugins?: unknown }).plugins;
      const plugin = findPlugin(plugins, RESOLVER_PLUGIN_NAME);
      expect(plugin).toBeDefined();
      expect(plugin?.enforce).toBe('pre');
      expect(typeof plugin?.resolveId).toBe('function');
    }
  });
});

const PROJECT_IMPORTER = '/fake/project/app.test.ts';

type ResolveResult = { id: string } | null;
type ResolveIdCtx = {
  resolve: (
    id: string,
    importer: string | undefined,
    options: Record<string, unknown>,
  ) => Promise<ResolveResult>;
};
type ResolveId = (
  this: ResolveIdCtx,
  id: string,
  importer: string | undefined,
  options: Record<string, unknown>,
) => Promise<string | { id: string } | null | undefined>;

interface ResolveCall {
  id: string;
  importer: string | undefined;
  fromProject: boolean;
}

function getResolveId(): ResolveId {
  const result = defineConfig({}) as { plugins: unknown[] };
  const plugin = findPlugin(result.plugins, RESOLVER_PLUGIN_NAME);
  expect(plugin).toBeDefined();
  return plugin?.resolveId as ResolveId;
}

// The resolver re-resolves through `this.resolve` rooted at vite-plus's own
// (bundled) anchors BEFORE the project importer. A `this.resolve` whose
// `importer` is NOT the project importer is therefore a bundled-anchor probe;
// `importer === PROJECT_IMPORTER` is the last-resort project fallback.
function makeCtx(
  resolveFor: (call: ResolveCall) => ResolveResult,
): { ctx: ResolveIdCtx; calls: ResolveCall[] } {
  const calls: ResolveCall[] = [];
  const ctx: ResolveIdCtx = {
    resolve: async (id, importer) => {
      const call: ResolveCall = { id, importer, fromProject: importer === PROJECT_IMPORTER };
      calls.push(call);
      return resolveFor(call);
    },
  };
  return { ctx, calls };
}

describe('vitePlusVitestResolverPlugin resolveId (bundle-first)', () => {
  // The runner `vp test` spawns is the Vitest bundled with vite-plus (see
  // resolve-test.ts). For the run to use a SINGLE physical Vitest, every
  // `vitest` / `@vitest/*` import must resolve to that same bundled copy — even
  // when the project keeps its own `vitest` dependency that the default
  // resolver would otherwise prefer.
  it('resolves the vitest family from a bundled anchor, never the project importer', async () => {
    const resolveId = getResolveId();
    const BUNDLED = '/bundled/vite-plus/node_modules/vitest/dist/index.js';
    const PROJECT = '/fake/project/node_modules/vitest/dist/index.js';
    const { ctx, calls } = makeCtx(({ fromProject }) => ({ id: fromProject ? PROJECT : BUNDLED }));

    const id = idOf(await resolveId.call(ctx, 'vitest', PROJECT_IMPORTER, {}));

    expect(id).toBe(BUNDLED);
    // The first bundled anchor hit short-circuits before the project importer.
    expect(calls).toHaveLength(1);
    expect(calls[0].fromProject).toBe(false);
  });

  it('tries the vite-plus anchor then the vitest anchor for the nested @vitest/* family', async () => {
    const resolveId = getResolveId();
    const VITEST_ANCHORED = '/bundled/.pnpm/vitest/node_modules/@vitest/expect/dist/index.js';
    let anchorProbes = 0;
    const { ctx, calls } = makeCtx(({ fromProject }) => {
      if (fromProject) {
        return { id: '/fake/project/node_modules/@vitest/expect/dist/index.js' };
      }
      anchorProbes += 1;
      // vite-plus anchor misses (@vitest/expect is a dep of vitest, not vite-plus);
      // the second (vitest) anchor resolves it.
      return anchorProbes >= 2 ? { id: VITEST_ANCHORED } : null;
    });

    const id = idOf(await resolveId.call(ctx, '@vitest/expect', PROJECT_IMPORTER, {}));

    expect(id).toBe(VITEST_ANCHORED);
    expect(anchorProbes).toBe(2);
    // Resolved from a bundled anchor — never fell through to the project importer.
    expect(calls.some((c) => c.fromProject)).toBe(false);
  });

  it('falls back to the project importer only when every bundled anchor misses', async () => {
    const resolveId = getResolveId();
    const FALLBACK = '/fake/project/node_modules/@vitest/coverage-v8/dist/index.js';
    const { ctx, calls } = makeCtx(({ fromProject }) => (fromProject ? { id: FALLBACK } : null));

    const id = idOf(await resolveId.call(ctx, '@vitest/coverage-v8', PROJECT_IMPORTER, {}));

    expect(id).toBe(FALLBACK);
    // Bundled anchors were probed first, then the project importer as last resort.
    expect(calls.some((c) => !c.fromProject)).toBe(true);
    expect(calls.at(-1)?.fromProject).toBe(true);
  });

  // Regression for the require-condition CJS throw-stub: Vitest's `.` /
  // `./config` exports map the `require` condition to `index.cjs` / `config.cjs`,
  // which throw "Vitest cannot be imported … using require()". Routing through
  // `this.resolve` (ESM conditions) avoids them; a raw `require.resolve` would
  // not. This ctx emulates Vite's ESM resolver so the contrast is exercised.
  it('resolves vitest / vitest/config to the ESM entry, never the require-condition CJS stub', async () => {
    const resolveId = getResolveId();
    const ctx: ResolveIdCtx = {
      resolve: async (id) => {
        try {
          return { id: fileURLToPath(import.meta.resolve(id)) };
        } catch {
          return null;
        }
      },
    };

    for (const id of ['vitest', 'vitest/config']) {
      const resolved = idOf(await resolveId.call(ctx, id, PROJECT_IMPORTER, {}));
      expect(resolved, `${id} should resolve to a real entry`).toBeTruthy();
      expect(
        resolved?.endsWith('.cjs'),
        `${id} must resolve to the ESM entry, not a CJS throw-stub: ${resolved}`,
      ).toBe(false);
    }
  });

  it('ignores non-family specifiers (returns null, never calls this.resolve)', async () => {
    const resolveId = getResolveId();
    const { ctx, calls } = makeCtx(() => ({ id: '/should/not/be/used' }));

    expect(idOf(await resolveId.call(ctx, 'react', PROJECT_IMPORTER, {}))).toBeUndefined();
    expect(calls).toHaveLength(0);
  });

  it('ignores queried family ids (defers entirely to the default pipeline)', async () => {
    const resolveId = getResolveId();
    const { ctx, calls } = makeCtx(() => ({ id: '/should/not/be/used' }));

    expect(idOf(await resolveId.call(ctx, 'vitest?v=1', PROJECT_IMPORTER, {}))).toBeUndefined();
    expect(calls).toHaveLength(0);
  });
});

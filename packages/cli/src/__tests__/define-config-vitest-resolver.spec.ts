import { fileURLToPath } from 'node:url';

import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import { defineConfig, defineProject, isVitestFamilySpecifier } from '../define-config.ts';

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

  it('is injected by defineProject too (browser project configs)', () => {
    const result = defineProject({ test: { browser: { enabled: true } } }) as {
      plugins: unknown[];
    };
    const plugin = findPlugin(result.plugins, RESOLVER_PLUGIN_NAME);

    expect(plugin).toBeDefined();
    expect(plugin?.enforce).toBe('pre');
    expect(typeof plugin?.resolveId).toBe('function');
  });
});

const PROJECT_IMPORTER = '/fake/project/app.test.ts';

type ResolveResult = { id: string } | null;
type ResolveIdCtx = {
  // The resolver reads its browser-vs-node signal per-server from
  // `this.environment.getTopLevelConfig().plugins`. Optional so a context can
  // omit it to exercise the fail-safe (no environment → no-op).
  environment?: { getTopLevelConfig: () => { plugins?: unknown } };
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

function getPlugin(): Record<string, unknown> {
  const result = defineConfig({}) as { plugins: unknown[] };
  const plugin = findPlugin(result.plugins, RESOLVER_PLUGIN_NAME);
  expect(plugin).toBeDefined();
  return plugin as Record<string, unknown>;
}

// `@vitest/browser` injects these (all named `vitest:browser*`) into its browser
// Vite server only; their presence on the resolving server's top-level config is
// the browser-server-exclusive signal the resolver gates on.
const BROWSER_SERVER_PLUGINS = [{ name: 'vitest:browser' }, { name: 'vitest:browser:tests' }];
// The node-side project server's plugins (`vitest:resolve-root` is vitest's own
// family resolver). No `vitest:browser*` member, so the resolver must no-op here.
const NODE_SERVER_PLUGINS = [
  { name: 'vitest' },
  { name: 'vitest:project' },
  { name: 'vitest:resolve-root' },
];

function getResolveId(): ResolveId {
  return getPlugin().resolveId as ResolveId;
}

// The resolver reads its browser-vs-node signal PER-SERVER from
// `this.environment.getTopLevelConfig().plugins`. `makeCtx` builds a `this` for a
// single Vite server: `plugins` is that server's top-level plugin list (default:
// the browser server, so the bundle-first tests below exercise the active path).
// A `this.resolve` whose `importer` is NOT the project importer is a bundled-anchor
// probe; `importer === PROJECT_IMPORTER` is the last-resort project fallback.
function makeCtx(
  resolveFor: (call: ResolveCall) => ResolveResult,
  plugins: ReadonlyArray<{ name: string }> = BROWSER_SERVER_PLUGINS,
): {
  ctx: ResolveIdCtx;
  calls: ResolveCall[];
} {
  const calls: ResolveCall[] = [];
  const ctx: ResolveIdCtx = {
    environment: { getTopLevelConfig: () => ({ plugins: [...plugins] }) },
    resolve: async (id, importer) => {
      const call: ResolveCall = { id, importer, fromProject: importer === PROJECT_IMPORTER };
      calls.push(call);
      return resolveFor(call);
    },
  };
  return { ctx, calls };
}

describe('vitePlusVitestResolverPlugin resolveId (browser-server gate)', () => {
  // The resolver acts ONLY inside @vitest/browser's browser Vite server. Node
  // mode resolves the vitest family itself (externalized to the bundled runner);
  // intercepting it can pick a different physical copy than the runner under
  // Yarn's peer-divergent dual-copy layout. The signal is read per-server from
  // `this.environment.getTopLevelConfig().plugins` (a `vitest:browser*` plugin).

  it('no-ops on the node-side server (no vitest:browser* plugin), never calling this.resolve', async () => {
    const resolveId = getResolveId();
    const { ctx, calls } = makeCtx(() => ({ id: '/should/not/be/used' }), NODE_SERVER_PLUGINS);

    expect(idOf(await resolveId.call(ctx, 'vitest', PROJECT_IMPORTER, {}))).toBeUndefined();
    expect(calls).toHaveLength(0);
  });

  it('no-ops (fail-safe to node behavior) when no environment is available', async () => {
    const resolveId = getResolveId();
    const calls: ResolveCall[] = [];
    const ctx: ResolveIdCtx = {
      resolve: async (id, importer) => {
        calls.push({ id, importer, fromProject: importer === PROJECT_IMPORTER });
        return { id: '/should/not/be/used' };
      },
    };

    expect(idOf(await resolveId.call(ctx, 'vitest', PROJECT_IMPORTER, {}))).toBeUndefined();
    expect(calls).toHaveLength(0);
  });

  // Critical regression: in config-declared browser mode `test.browser.enabled`
  // is true on BOTH the node-side project server AND the browser server, but only
  // the browser server carries `vitest:browser*` plugins. The node server must
  // stay a strict no-op even when resolution arrives with ssr:false — gating on
  // `test.browser.enabled` would wrongly redirect the family here.
  it('stays a no-op on the node server even in config-declared browser mode (ssr:false, no browser plugins)', async () => {
    const resolveId = getResolveId();
    const { ctx, calls } = makeCtx(() => ({ id: '/should/not/be/used' }), NODE_SERVER_PLUGINS);

    expect(
      idOf(await resolveId.call(ctx, 'vitest', PROJECT_IMPORTER, { ssr: false })),
    ).toBeUndefined();
    expect(calls).toHaveLength(0);
  });

  // Regression for the node-mode DOM hole: vitest 4.x marks `jsdom`/`happy-dom`
  // as vite's `client` environment, so a NODE-mode test in those environments
  // resolves the family with ssr:false even though browser mode is OFF. Its server
  // has no `vitest:browser*` plugin, so the resolver must stay a strict no-op —
  // gating on ssr:false would reopen the dual-copy split.
  it('stays a no-op for a node-mode jsdom server (ssr:false, no browser plugins)', async () => {
    const resolveId = getResolveId();
    const { ctx, calls } = makeCtx(() => ({ id: '/should/not/be/used' }), NODE_SERVER_PLUGINS);

    expect(
      idOf(await resolveId.call(ctx, 'vitest', PROJECT_IMPORTER, { ssr: false })),
    ).toBeUndefined();
    expect(calls).toHaveLength(0);
  });

  // The `--browser` CLI flag never writes `test.browser.enabled` back to the
  // resolved config, but @vitest/browser still injects its `vitest:browser*`
  // plugins into the browser server under both that path and the config-declared
  // path — so their presence on the resolving server activates the resolver.
  it('activates when the browser server carries @vitest/browser plugins', async () => {
    const resolveId = getResolveId();
    const BUNDLED = '/bundled/vite-plus/node_modules/vitest/dist/index.js';
    const { ctx, calls } = makeCtx(
      ({ fromProject }) => ({
        id: fromProject ? '/fake/project/node_modules/vitest/dist/index.js' : BUNDLED,
      }),
      BROWSER_SERVER_PLUGINS,
    );

    expect(idOf(await resolveId.call(ctx, 'vitest', PROJECT_IMPORTER, { ssr: false }))).toBe(
      BUNDLED,
    );
    expect(calls[0]?.fromProject).toBe(false);
  });

  // Critical regression for the shared-plugin-instance leak: vitest reuses ONE
  // copy of this plugin object across the node-side AND browser servers. The gate
  // is read per-resolution from `this.environment`, so a browser-server
  // resolution that activates must NOT leak into a later node-server resolution
  // on the SAME plugin object.
  it('is per-server: a node-server resolution no-ops even after a browser-server resolution activated', async () => {
    const resolveId = getResolveId(); // ONE plugin instance, reused for both calls below.
    const BUNDLED = '/bundled/vite-plus/node_modules/vitest/dist/index.js';

    // 1) browser-server `this` → activates and redirects to the bundled copy.
    const browser = makeCtx(
      ({ fromProject }) => ({ id: fromProject ? '/fake/project/vitest' : BUNDLED }),
      BROWSER_SERVER_PLUGINS,
    );
    expect(
      idOf(await resolveId.call(browser.ctx, 'vitest', PROJECT_IMPORTER, { ssr: false })),
    ).toBe(BUNDLED);

    // 2) node-server `this` on the SAME plugin → must stay a no-op (no leaked state).
    const node = makeCtx(() => ({ id: '/should/not/be/used' }), NODE_SERVER_PLUGINS);
    expect(
      idOf(await resolveId.call(node.ctx, 'vitest', PROJECT_IMPORTER, { ssr: false })),
    ).toBeUndefined();
    expect(node.calls).toHaveLength(0);
  });
});

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
      environment: { getTopLevelConfig: () => ({ plugins: [...BROWSER_SERVER_PLUGINS] }) },
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

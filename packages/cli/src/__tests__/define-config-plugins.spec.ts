import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import {
  assertCoverageProviderVersionMatch,
  AUTO_INLINE_DEPS,
  checkCoverageProviderVersion,
  computeAutoInlineList,
  defineConfig,
  resolveCoverageProviderToCheck,
} from '../define-config.ts';
import { VITEST_VERSION } from '../utils/constants.ts';

const RESOLVER_PLUGIN_NAME = 'vite-plus:vitest-resolver';
const COVERAGE_GUARD_PLUGIN_NAME = 'vite-plus:coverage-version-guard';

function pluginName(p: unknown): string | undefined {
  if (p && typeof p === 'object' && 'name' in p && typeof p.name === 'string') {
    return (p as { name: string }).name;
  }
  return undefined;
}

describe('defineConfig project plugin injection', () => {
  it('injects resolver + auto-inline plugins at the root plugins array', () => {
    const existing: Plugin = { name: 'user-existing-root-plugin' };
    const result = defineConfig({ plugins: [existing] }) as { plugins: unknown[] };

    expect(pluginName(result.plugins[0])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(result.plugins[1])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(result.plugins[2])).toBe(COVERAGE_GUARD_PLUGIN_NAME);
    expect(pluginName(result.plugins[3])).toBe('user-existing-root-plugin');
  });

  it('injects resolver + auto-inline plugins into an inline-object project entry, preserving existing plugins', () => {
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
    expect(pluginName(project.plugins[0])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(project.plugins[1])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(project.plugins[2])).toBe(COVERAGE_GUARD_PLUGIN_NAME);
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
    expect(pluginName(resolved.plugins[0])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(resolved.plugins[1])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(resolved.plugins[2])).toBe(COVERAGE_GUARD_PLUGIN_NAME);
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
    expect(pluginName(project.plugins[0])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(project.plugins[1])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(project.plugins[2])).toBe(COVERAGE_GUARD_PLUGIN_NAME);
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
    const capturedFromPaths: string[] = [];
    const factory = (from: string) => {
      capturedFromPaths.push(from);
      return { resolve: (_id: string) => `/mock/node_modules/${_id}/index.js` };
    };
    computeAutoInlineList(undefined, '/custom/root', factory);
    expect(capturedFromPaths).toEqual(['/custom/root/package.json']);
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

/**
 * Build injectable deps for {@link checkCoverageProviderVersion}: only the
 * packages in `installed` resolve, and reading their package.json returns the
 * mapped version.
 */
function makeCoverageDeps(installed: Record<string, string>): {
  createRequire: (from: string) => { resolve: (id: string) => string };
  readFile: (path: string) => string;
} {
  return {
    createRequire: (_from: string) => ({
      resolve(id: string) {
        const pkg = id.replace(/\/package\.json$/, '');
        if (pkg in installed) {
          return `/mock/node_modules/${pkg}/package.json`;
        }
        throw new Error(`Cannot find module '${id}'`);
      },
    }),
    readFile(path: string) {
      const pkg = path.replace(/^\/mock\/node_modules\//, '').replace(/\/package\.json$/, '');
      return JSON.stringify({ version: installed[pkg] });
    },
  };
}

describe('resolveCoverageProviderToCheck', () => {
  // Input is Vitest's RESOLVED coverage options (CLI flags already merged by
  // Vitest), so the helper never parses argv itself.
  it('returns @vitest/coverage-v8 for enabled coverage with the default provider', () => {
    expect(resolveCoverageProviderToCheck({ enabled: true })).toBe('@vitest/coverage-v8');
  });

  it('honors an explicit istanbul provider', () => {
    expect(resolveCoverageProviderToCheck({ enabled: true, provider: 'istanbul' })).toBe(
      '@vitest/coverage-istanbul',
    );
  });

  it('returns null when coverage is off or undefined', () => {
    expect(resolveCoverageProviderToCheck(undefined)).toBeNull();
    expect(resolveCoverageProviderToCheck({ enabled: false })).toBeNull();
    expect(resolveCoverageProviderToCheck({ provider: 'v8' })).toBeNull();
  });

  it('returns null for a custom/unknown provider vite-plus does not bundle a runner for', () => {
    expect(resolveCoverageProviderToCheck({ enabled: true, provider: 'custom' })).toBeNull();
  });
});

describe('assertCoverageProviderVersionMatch', () => {
  it('does not throw when the provider version matches the bundled runner', () => {
    expect(() =>
      assertCoverageProviderVersionMatch('@vitest/coverage-v8', VITEST_VERSION),
    ).not.toThrow();
  });

  it('throws when the provider version is skewed from the bundled runner', () => {
    expect(() => assertCoverageProviderVersionMatch('@vitest/coverage-v8', '4.1.8')).toThrow(
      /bundles vitest@/,
    );
  });

  it('does not throw when the provider version is unknown (not installed)', () => {
    expect(() => assertCoverageProviderVersionMatch('@vitest/coverage-v8', null)).not.toThrow();
    expect(() =>
      assertCoverageProviderVersionMatch('@vitest/coverage-v8', undefined),
    ).not.toThrow();
  });
});

describe('checkCoverageProviderVersion', () => {
  it('throws when coverage is on and the installed provider is version-skewed', () => {
    const deps = makeCoverageDeps({ '@vitest/coverage-v8': '4.1.8' });
    expect(() => checkCoverageProviderVersion({ enabled: true }, '/project', deps)).toThrow(
      /@vitest\/coverage-v8@4\.1\.8/,
    );
  });

  it('does not throw when the installed provider matches the bundled runner', () => {
    const deps = makeCoverageDeps({ '@vitest/coverage-v8': VITEST_VERSION });
    expect(() => checkCoverageProviderVersion({ enabled: true }, '/project', deps)).not.toThrow();
  });

  it('is a no-op when coverage is off, even if a skewed provider is installed', () => {
    const deps = makeCoverageDeps({ '@vitest/coverage-v8': '4.1.8' });
    expect(() => checkCoverageProviderVersion(undefined, '/project', deps)).not.toThrow();
  });

  it('is a no-op when the provider is not installed (vitest emits its own error)', () => {
    const deps = makeCoverageDeps({});
    expect(() => checkCoverageProviderVersion({ enabled: true }, '/project', deps)).not.toThrow();
  });

  it('checks the istanbul provider when it is the selected provider', () => {
    const deps = makeCoverageDeps({ '@vitest/coverage-istanbul': '4.1.8' });
    expect(() =>
      checkCoverageProviderVersion({ enabled: true, provider: 'istanbul' }, '/project', deps),
    ).toThrow(/@vitest\/coverage-istanbul@4\.1\.8/);
  });

  it('falls back to the bundled-vitest anchor when the project root cannot resolve the provider', () => {
    // pnpm peer-set / hoisted layouts: the provider is loadable by vitest (next
    // to the bundled vitest) but is NOT resolvable from the project root. The
    // guard must still catch the skew via the bundled-vitest fallback anchor
    // instead of silently skipping. The mock fails resolution from the project
    // root and only resolves from any other anchor.
    const projectRootAnchor = '/project/package.json';
    const deps = {
      createRequire: (from: string) => ({
        resolve(id: string) {
          if (from === projectRootAnchor) {
            throw new Error('provider not reachable from the project root');
          }
          return `/mock/node_modules/${id.replace(/\/package\.json$/, '')}/package.json`;
        },
      }),
      readFile: () => JSON.stringify({ version: '4.1.8' }),
    };
    expect(() => checkCoverageProviderVersion({ enabled: true }, '/project', deps)).toThrow(
      /@vitest\/coverage-v8@4\.1\.8/,
    );
  });
});

/** Extract the coverage-version-guard plugin from a fresh defineConfig. */
function getCoverageGuard(): { configureVitest: (ctx: unknown) => void } {
  const result = defineConfig({}) as { plugins: unknown[] };
  const plugin = result.plugins.find(
    (p): p is { configureVitest: (ctx: unknown) => void } =>
      !!p && typeof p === 'object' && (p as { name?: unknown }).name === COVERAGE_GUARD_PLUGIN_NAME,
  );
  if (!plugin) {
    throw new Error('coverage guard plugin not found');
  }
  return plugin;
}

describe('defineConfig coverage version guard plugin registration', () => {
  it('registers the coverage guard plugin with a configureVitest hook', () => {
    const result = defineConfig({}) as { plugins: unknown[] };
    const plugin = result.plugins.find(
      (p): p is Record<string, unknown> =>
        !!p &&
        typeof p === 'object' &&
        (p as { name?: unknown }).name === COVERAGE_GUARD_PLUGIN_NAME,
    );
    expect(plugin).toBeDefined();
    expect(typeof plugin?.configureVitest).toBe('function');
  });

  it('configureVitest is a no-op when the runner coverage is off (startup path)', () => {
    const guard = getCoverageGuard();
    // The hook short-circuits before any resolution when coverage is disabled,
    // so a non-existent root is fine.
    expect(() =>
      guard.configureVitest({
        vitest: {
          config: { root: '/project', coverage: { enabled: false } },
          enableCoverage: async () => {},
        },
      }),
    ).not.toThrow();
  });

  it('wraps the shared vitest.enableCoverage so the late path runs the check, and still delegates', async () => {
    const guard = getCoverageGuard();
    let originalCalls = 0;
    const original = async () => {
      originalCalls += 1;
    };
    const vitest: Record<string, unknown> = {
      config: { root: '/project', coverage: { provider: 'v8' } },
      enableCoverage: original,
    };
    guard.configureVitest({ vitest });
    // enableCoverage was replaced by the guard's wrapper...
    expect(vitest.enableCoverage).not.toBe(original);
    // ...and the wrapper delegates to the original (no provider resolves from
    // this fake root, so the late-path check is a no-op).
    await (vitest.enableCoverage as () => Promise<void>)();
    expect(originalCalls).toBe(1);
  });

  it('handles the shared vitest instance once even though configureVitest runs per project', () => {
    const guard = getCoverageGuard();
    const vitest: Record<string, unknown> = {
      config: { root: '/project', coverage: {} },
      enableCoverage: async () => {},
    };
    // The hook fires once per project but `vitest` is shared; the guard reads the
    // global runner config (not per-project) and wraps the instance only once.
    guard.configureVitest({ vitest });
    const afterFirst = vitest.enableCoverage;
    guard.configureVitest({ vitest });
    expect(vitest.enableCoverage).toBe(afterFirst);
  });
});

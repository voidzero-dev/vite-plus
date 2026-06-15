import type { Plugin } from '@voidzero-dev/vite-plus-core';
import { describe, expect, it } from 'vitest';

import { AUTO_INLINE_DEPS, computeAutoInlineList, defineConfig } from '../define-config.ts';

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

describe('defineConfig project plugin injection', () => {
  it('injects resolver + auto-inline plugins at the root plugins array', () => {
    const existing: Plugin = { name: 'user-existing-root-plugin' };
    const result = defineConfig({ plugins: [existing] }) as { plugins: unknown[] };

    expect(pluginName(result.plugins[0])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(result.plugins[1])).toBe(AUTO_INLINE_PLUGIN_NAME);
    expect(pluginName(result.plugins[2])).toBe('user-existing-root-plugin');
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
    expect(pluginName(project.plugins[2])).toBe('user-unit-project-plugin');
    // Sanity: the existing plugin reference is preserved (clone shallow-copies the array).
    expect(project.plugins[2]).toBe(existing);
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
    expect(pluginName(project.plugins[0])).toBe(RESOLVER_PLUGIN_NAME);
    expect(pluginName(project.plugins[1])).toBe(AUTO_INLINE_PLUGIN_NAME);
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

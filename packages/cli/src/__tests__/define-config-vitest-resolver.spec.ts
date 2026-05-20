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

  it('is injected into each `test.projects` entry', () => {
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

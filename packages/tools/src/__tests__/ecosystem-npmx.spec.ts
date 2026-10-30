import { runInNewContext } from 'node:vm';

import { describe, expect, test } from 'vitest';

import { patchNpmxVitestConfig } from '../../../../ecosystem-ci/npmx.ts';

const nuxtProject = `defineVitestProject({
  plugins: [liveDollarFetch()],
  resolve: { alias: { '@vue/compiler-dom': 'compiler-dom.esm-browser.js' } },
  test: { name: 'nuxt', browser: { enabled: true } },
})`;

describe('patchNpmxVitestConfig', () => {
  test.each([
    nuxtProject,
    nuxtProject.replace('[liveDollarFetch()]', '[\n    liveDollarFetch(),\n  ]'),
  ])('preserves the existing plugin, aliases, and test settings', (config) => {
    const liveDollarFetch = { name: 'live-dollar-fetch' };
    const patched = runInNewContext(patchNpmxVitestConfig(config), {
      defineVitestProject: (project: unknown) => project,
      liveDollarFetch: () => liveDollarFetch,
    }) as {
      plugins: {
        name: string;
        config?: () => { test: { browser: { importMapMocks: boolean } } };
      }[];
      resolve: { alias: Record<string, string> };
      test: { name: string; browser: { enabled: boolean } };
    };

    expect(patched.plugins).toHaveLength(2);
    expect(patched.plugins[0]).toBe(liveDollarFetch);
    expect(patched.plugins[1].name).toBe('npmx:test:vitest-compat');
    expect(patched.plugins[1].config?.()).toEqual({
      test: { browser: { importMapMocks: false } },
    });
    expect(patched.test).toEqual({ name: 'nuxt', browser: { enabled: true } });
    expect(patched.resolve.alias).toEqual({
      '@vue/compiler-dom': 'compiler-dom.esm-browser.js',
    });
  });

  test('leaves the root and other projects unchanged', () => {
    const prefix = `export default defineConfig({
  plugins: [rootPlugin()],
  test: { projects: [
    { plugins: [unitPlugin()], test: { name: 'unit' } },
    () => `;
    const suffix = ',\n  ] },\n})';
    expect(patchNpmxVitestConfig(prefix + nuxtProject + suffix)).toBe(
      prefix + patchNpmxVitestConfig(nuxtProject) + suffix,
    );
  });

  test.each([
    '',
    nuxtProject.replace('  plugins: [liveDollarFetch()],\n', ''),
    nuxtProject.replace('liveDollarFetch()', 'anotherPlugin()'),
    `${nuxtProject}\n${nuxtProject}`,
  ])('rejects an unexpected or ambiguous fixture layout', (config) => {
    expect(() => patchNpmxVitestConfig(config)).toThrow(
      'npmx.dev patch: expected the pinned Nuxt test project configuration',
    );
  });

  test('rejects an already patched config instead of duplicating the workaround', () => {
    expect(() => patchNpmxVitestConfig(patchNpmxVitestConfig(nuxtProject))).toThrow(
      'npmx.dev patch: expected the pinned Nuxt test project configuration',
    );
  });
});

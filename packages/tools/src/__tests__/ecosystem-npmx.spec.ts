import { runInNewContext } from 'node:vm';

import { describe, expect, test } from 'vitest';

import { patchNpmxVitestConfig } from '../../../../ecosystem-ci/npmx.ts';

const nuxtProject = `defineVitestProject({
  plugins: [liveDollarFetch()],
  test: { name: 'nuxt', browser: { enabled: true } },
})`;

interface OptimizerOptions {
  include?: string[];
  exclude?: string[];
}

interface TestServer {
  config: { optimizeDeps: OptimizerOptions };
  environments: { client: { config: { optimizeDeps: OptimizerOptions } } };
}

describe('patchNpmxVitestConfig', () => {
  test.each([
    nuxtProject,
    nuxtProject.replace('[liveDollarFetch()]', '[\n    liveDollarFetch(),\n  ]'),
  ])('preserves the existing plugin and test settings', (config) => {
    const liveDollarFetch = { name: 'live-dollar-fetch' };
    const patched = runInNewContext(patchNpmxVitestConfig(config), {
      defineVitestProject: (project: unknown) => project,
      liveDollarFetch: () => liveDollarFetch,
    }) as {
      plugins: { name: string; configureServer?: (server: TestServer) => void }[];
      test: { name: string; browser: { enabled: boolean } };
    };

    expect(patched.plugins).toHaveLength(2);
    expect(patched.plugins[0]).toBe(liveDollarFetch);
    expect(patched.plugins[1].name).toBe('npmx:test:preserve-optimizer-exclusions');
    expect(patched.test).toEqual({ name: 'nuxt', browser: { enabled: true } });

    const server: TestServer = {
      config: { optimizeDeps: { include: ['keep', 'excluded'], exclude: ['excluded'] } },
      environments: {
        client: {
          config: {
            optimizeDeps: { include: ['browser', 'excluded'], exclude: ['excluded'] },
          },
        },
      },
    };
    patched.plugins[1].configureServer?.(server);
    expect(server.config.optimizeDeps).toEqual({ include: ['keep'], exclude: ['excluded'] });
    expect(server.environments.client.config.optimizeDeps).toEqual({
      include: ['browser'],
      exclude: ['excluded'],
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

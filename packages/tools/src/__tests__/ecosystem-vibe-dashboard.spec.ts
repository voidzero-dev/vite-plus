import { runInNewContext } from 'node:vm';

import { expect, test } from 'vite-plus/test';

import { patchVibeDashboardConfig } from '../../../../ecosystem-ci/vibe-dashboard.ts';

const browserProject = `defineConfig({
  plugins: lazyPlugins(() => [tailwindcss()]),
  test: { browser: { enabled: true } },
})`;

interface TestConfig {
  test?: { api?: boolean | { port: number; host?: string } };
}

test.each([browserProject, browserProject.replace('[tailwindcss()]', '[\n  tailwindcss(),\n]')])(
  'preserves migrated plugins and selects the port after Vitest defaults',
  (config) => {
    const tailwind = { name: 'tailwind' };
    const patched = runInNewContext(patchVibeDashboardConfig(config), {
      defineConfig: (project: unknown) => project,
      tailwindcss: () => tailwind,
      lazyPlugins: (factory: () => unknown[]) => factory(),
    }) as {
      plugins: {
        enforce?: string;
        config?: { order: string; handler: (config: TestConfig) => void };
      }[];
      test: { browser: { enabled: boolean } };
    };

    expect(patched.plugins).toHaveLength(2);
    expect(patched.plugins[0]).toBe(tailwind);
    expect(patched.test).toEqual({ browser: { enabled: true } });
    const plugin = patched.plugins[1];
    expect(plugin.enforce).toBe('post');
    expect(plugin.config?.order).toBe('post');

    const resolved = { test: { api: { port: 63315, host: 'localhost' } } };
    plugin.config?.handler(resolved);
    expect(resolved.test.api).toEqual({ port: 0, host: 'localhost' });

    // A normal Vite build has no resolved Vitest API options.
    for (const buildConfig of [{}, { test: {} }, { test: { api: false } }]) {
      const original = structuredClone(buildConfig);
      plugin.config?.handler(buildConfig);
      expect(buildConfig).toEqual(original);
    }
  },
);

test.each([
  browserProject.replace('tailwindcss()', 'otherPlugin()'),
  `${browserProject}\n${browserProject}`,
  patchVibeDashboardConfig(browserProject),
])('rejects an unexpected, ambiguous, or already patched project', (config) => {
  expect(() => patchVibeDashboardConfig(config)).toThrow(
    'vibe-dashboard patch: expected the pinned browser project plugins',
  );
});

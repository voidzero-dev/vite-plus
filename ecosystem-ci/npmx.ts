export function patchNpmxVitestConfig(config: string): string {
  // The pinned Nuxt project uses liveDollarFetch to avoid capturing a stale
  // $fetch during browser tests. Keep it and the upstream Vue compiler alias.
  const nuxtProject = /defineVitestProject\(\{\s*plugins:\s*\[\s*liveDollarFetch\(\)\s*,?\s*\]/g;
  if ([...config.matchAll(nuxtProject)].length !== 1) {
    throw new Error('npmx.dev patch: expected the pinned Nuxt test project configuration');
  }
  return config.replace(
    nuxtProject,
    `defineVitestProject({
          plugins: [liveDollarFetch(), {
            name: 'npmx:test:vitest-compat',
            config() {
              // Nuxt imports app modules before per-file vi.mock calls run.
              // Use request interception so those modules can still be mocked.
              // https://vitest.dev/config/browser/importmapmocks
              return { test: { browser: { importMapMocks: false } } }
            },
          }]`,
  );
}

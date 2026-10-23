export function patchNpmxVitestConfig(config: string): string {
  // The pinned Nuxt project uses liveDollarFetch to avoid capturing a stale
  // $fetch during browser tests. Keep it when adding our Vitest workarounds.
  const nuxtProject =
    /defineVitestProject\(\{\s*plugins:\s*\[\s*liveDollarFetch\(\)\s*,?\s*\],\s*test:\s*\{/g;
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
            // Temporary Vitest 5 workaround: preserve Nuxt's optimizer exclusions.
            // Remove when upstream preserves these exclusions after the merge.
            // https://github.com/why-reproductions-are-required/vitest-browser-optimizer-config-order
            configureServer(server) {
              for (const options of [
                server.config.optimizeDeps,
                server.environments.client.config.optimizeDeps,
              ]) {
                const excluded = new Set(options.exclude ?? [])
                options.include = options.include?.filter(dep => !excluded.has(dep))
              }
            },
          }],
          test: {`,
  );
}

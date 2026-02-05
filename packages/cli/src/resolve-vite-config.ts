export async function resolveUniversalViteConfig(err: null | Error, viteConfigCwd: string) {
  if (err) {
    throw err;
  }
  try {
    const { resolveConfig } = await import('./index.js');
    const config = await resolveConfig({ root: viteConfigCwd }, 'build');

    return Promise.resolve(
      JSON.stringify({
        configFile: config.configFile,
        lint: config.lint,
        fmt: config.fmt,
        run: config.run,
      }),
    );
  } catch (err) {
    console.error('[Vite+] resolve universal vite config error:', err);
    throw err;
  }
}

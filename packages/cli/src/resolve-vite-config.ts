export async function resolveUniversalViteConfig(err: null | Error, viteConfigCwd: string) {
  if (err) {
    throw err;
  }
  try {
    const { resolveConfig } = await import('./index.js');
    const config = await resolveConfig({ root: viteConfigCwd }, 'build');

    return Promise.resolve(
      JSON.stringify({
        lint: config.lint,
        fmt: config.fmt,
        tasks: config.tasks,
      }),
    );
  } catch (err) {
    console.error('[vite+] resolve universal vite config error:', err);
    throw err;
  }
}

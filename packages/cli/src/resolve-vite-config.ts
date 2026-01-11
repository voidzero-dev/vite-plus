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
    // Return empty config when loading fails (e.g., unbuilt workspace packages)
    // This allows the task runner to continue with other packages
    console.error(`failed to load config from ${viteConfigCwd}/vite.config.ts`);
    return Promise.resolve(JSON.stringify({
      lint: null,
      fmt: null,
      tasks: null,
    }));
  }
}

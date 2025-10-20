import pkgJson from '../package.json' with { type: 'json' };

export const RewriteImportsPlugin = {
  name: 'externalize-vite-and-rolldown',
  resolveId(id: string) {
    if (id.startsWith('vite/')) {
      return { id: id.replace(/^vite\//, `${pkgJson.name}/`), external: true };
    }
    if (id === 'rolldown') {
      return { id: `${pkgJson.name}/rolldown`, external: true };
    }
    if (id.startsWith('rolldown/')) {
      return { id: id.replace(/^rolldown\//, `${pkgJson.name}/rolldown/`), external: true };
    }
  },
};

import { fileURLToPath } from 'node:url';

import type { Plugin } from 'rolldown';

/** Remove with the temporary #11198 backport after upgrading Vitest. */
export function vitestBrowserDefinesBackportBuildPlugin(vitestVersion: string): Plugin | false {
  // Do not ship the workaround when sync-remote selects a newer Vitest release.
  if (vitestVersion !== '5.0.0') {
    return false;
  }
  const runtimePath = fileURLToPath(
    new URL('../src/vitest-browser-defines-backport.ts', import.meta.url),
  );
  const anchor = '    optimizedDepsPlugin(),';
  return {
    name: 'backport-vitest-browser-defines',
    transform(code, id, { magicString }) {
      if (!id.replaceAll('\\', '/').endsWith('/vite/src/node/plugins/index.ts')) {
        return undefined;
      }
      if (!magicString || code.split(anchor).length !== 2) {
        throw new Error('Cannot inject the temporary Vitest #11198 backport into Vite.');
      }
      magicString.prepend(
        `import { vitestBrowserDefinesBackportPlugin } from ${JSON.stringify(runtimePath)};\n`,
      );
      magicString.replace(
        anchor,
        `${anchor}\n    !isBuild && !isWorker ? vitestBrowserDefinesBackportPlugin() : null,`,
      );
      return { code: magicString };
    },
  };
}

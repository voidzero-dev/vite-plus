import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);

export async function build(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  const pkgJsonPath = require.resolve('vite/package.json');
  const binPath = join(dirname(pkgJsonPath), 'bin', 'vite.js');
  return {
    binPath,
    envs: process.env.DEBUG_DISABLE_SOURCE_MAP
      ? {
        DEBUG_DISABLE_SOURCE_MAP: process.env.DEBUG_DISABLE_SOURCE_MAP,
      }
      : {},
  };
}

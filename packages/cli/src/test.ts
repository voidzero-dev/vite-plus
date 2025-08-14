import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

export async function test(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  const binPath = require.resolve('vitest/vitest.mjs');
  return {
    binPath,
    envs: process.env.DEBUG_DISABLE_SOURCE_MAP
      ? {
        DEBUG_DISABLE_SOURCE_MAP: process.env.DEBUG_DISABLE_SOURCE_MAP,
      }
      : {},
  };
}

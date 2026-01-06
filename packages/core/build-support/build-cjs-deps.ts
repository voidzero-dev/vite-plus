import { writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';

import { build } from 'rolldown';

export function createModuleEntryFileName(module: string) {
  // remove the .js extension in the require path
  // like `require('semver/functions/coerce.js') -> npm_entry_semver_functions_coerce.cjs`
  return `npm_entry_${module.replaceAll('/', '_').replace('.js', '')}.cjs`;
}

export async function buildCjsDeps(modules: Set<string>, distDir: string) {
  const distFiles = new Set<string>();
  for (const module of modules) {
    const filename = createModuleEntryFileName(module);
    const distFile = join(distDir, `_${filename}`);
    await writeFile(distFile, `module.exports = require('${module}')\n`);
    distFiles.add(distFile);
  }
  if (distFiles.size === 0) {
    return;
  }
  await build({
    input: Array.from(distFiles),
    platform: 'node',
    treeshake: true,
    output: {
      format: 'cjs',
      dir: distDir,
      entryFileNames: (chunkInfo) => {
        return `${chunkInfo.name.slice(1)}.cjs`;
      },
      chunkFileNames: (chunkInfo) => {
        return `npm_cjs_chunk_${chunkInfo.name || 'index'}.cjs`;
      },
    },
  });

  for (const file of distFiles) {
    await rm(file);
  }
}

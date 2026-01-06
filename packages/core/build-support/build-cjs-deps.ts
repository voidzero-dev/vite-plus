import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { build } from 'rolldown';

export function createModuleEntryFileName(module: string) {
  return `npm_entry_${module.replaceAll('/', '_')}.cjs`;
}

export async function buildCjsDeps(modules: Set<string>, distDir: string) {
  const distFiles = new Set<string>();
  for (const module of modules) {
    const filename = createModuleEntryFileName(module);
    const distFile = join(distDir, filename);
    await writeFile(distFile, `module.exports = require('${module}')\n`);
    distFiles.add(distFile);
  }
  await build({
    input: Array.from(distFiles),
    platform: 'node',
    treeshake: true,
    output: {
      format: 'cjs',
      dir: join(distDir, 'npm-cjs-deps'),
    },
  });
}

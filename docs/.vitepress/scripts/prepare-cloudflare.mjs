import { realpath } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import {
  cleanBuildOutputDir,
  readBuildOutput,
  writeAssets,
  writeRootConfig,
  writeWorkerConfig,
} from '@cloudflare/build-output-utils';

import config from '../../cloudflare.config.ts';

const docsRoot = fileURLToPath(new URL('../../', import.meta.url));

export async function prepareCloudflare(sourceDirectory, root = docsRoot) {
  // VitePress already built the site. Package only its assets for cf --prebuilt;
  // never load configuration or Worker code from the asset directory.
  await cleanBuildOutputDir(root);
  await writeRootConfig(root, undefined, { isPreview: false });
  await writeWorkerConfig({ root, config: config.worker });
  await writeAssets({ root, sourceDirectory });
  await readBuildOutput(root);
}

if (process.argv[1] && import.meta.url === pathToFileURL(await realpath(process.argv[1])).href) {
  await prepareCloudflare(resolve(process.argv[2] ?? resolve(docsRoot, '.vitepress/dist')));
}

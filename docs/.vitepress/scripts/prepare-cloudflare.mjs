import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  cleanBuildOutputDir,
  readBuildOutput,
  writeAssets,
  writeRootConfig,
  writeWorkerConfig,
} from '@cloudflare/build-output-utils';

import config from '../../cloudflare.config.ts';
import { resolveWorkersPreviewBranch } from '../site-origin.ts';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sourceDirectory = resolve(process.argv[2] ?? resolve(root, '.vitepress/dist'));

// VitePress already built the site. Package only its assets for cf --prebuilt;
// never load configuration or Worker code from the asset directory.
await cleanBuildOutputDir(root);
// `cf previews deploy --prebuilt` accepts only Preview output, and
// `cf deploy --prebuilt` rejects it.
await writeRootConfig(root, undefined, { isPreview: resolveWorkersPreviewBranch() !== undefined });
await writeWorkerConfig({ root, config: config.worker });
await writeAssets({ root, sourceDirectory });
await readBuildOutput(root);

import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectDir = dirname(fileURLToPath(import.meta.url));

// Use RUNNER_TEMP in GitHub Actions, otherwise use system temp directory
const tempBase = process.env.RUNNER_TEMP ?? tmpdir();

export const ecosystemCiDir = join(tempBase, 'vite-plus-ecosystem-ci');

// tgz path: always use local tmp/tgz
export const tgzDir = join(projectDir, '..', 'tmp', 'tgz');

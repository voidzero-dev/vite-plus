import { execSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import repos from './repo.json' with { type: 'json' };

const scriptDir = dirname(fileURLToPath(import.meta.url));

function getEcosystemCiDir(): string {
  if (process.env.ECOSYSTEM_CI_DIR) {
    return process.env.ECOSYSTEM_CI_DIR;
  }
  // Use realpathSync for macOS where tmpdir() returns a symlink
  return join(realpathSync(tmpdir()), 'vite-plus-e2e');
}

const ecosystemCiDir = getEcosystemCiDir();

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

// tgzPath stays relative to repo (where the built packages are)
const tgzPath = join(scriptDir, '..', 'tmp', 'tgz');

async function migrateProject(project: string) {
  const repoRoot = join(ecosystemCiDir, project);
  // run vite migrate
  execSync('vite migrate', {
    cwd: repoRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      VITE_PLUS_OVERRIDE_PACKAGES: JSON.stringify({
        vite: `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
        vitest: `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
        '@voidzero-dev/vite-plus-core': `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
        '@voidzero-dev/vite-plus-test': `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
      }),
      VITE_PLUS_VERSION: `file:${tgzPath}/vite-plus-0.0.0.tgz`,
    },
  });
}

await migrateProject(project);

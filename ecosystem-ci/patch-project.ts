import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import repos from './repo.json' with { type: 'json' };

const projectDir = dirname(fileURLToPath(import.meta.url));

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const tgzPath = join(projectDir, '..', 'tmp', 'tgz');

async function migrateProject(project: string) {
  const repoRoot = join(projectDir, project);
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
      VITE_PLUS_VERSION: `file:${tgzPath}/voidzero-dev-vite-plus-0.0.0.tgz`,
    },
  });

  // Post-migration fixes for specific projects
  if (project === 'vite-plugin-react') {
    // Remove plugin-react-swc playgrounds from workspace
    // These playgrounds have vite configs that use SWC plugins which can't be resolved
    // due to pnpm's strict node_modules isolation
    const workspaceYamlPath = join(repoRoot, 'pnpm-workspace.yaml');
    let workspaceYaml = readFileSync(workspaceYamlPath, 'utf-8');
    workspaceYaml = workspaceYaml.replace(
      "  - 'packages/plugin-react-swc/playground/**'\n",
      '',
    );
    writeFileSync(workspaceYamlPath, workspaceYaml);
    console.log('Removed plugin-react-swc playgrounds from workspace');
  }
}

await migrateProject(project);

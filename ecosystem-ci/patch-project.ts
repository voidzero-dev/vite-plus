import { execSync } from 'node:child_process';
import fs from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import vitestPackageJson from '../packages/test/package.json' with { type: 'json' };
import repos from './repo.json' with { type: 'json' };

const projectDir = dirname(fileURLToPath(import.meta.url));

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const tgzPath = join(projectDir, '..', 'tmp', 'tgz');

async function patchVibeDashboard() {
  const pnpmWorkspacePath = join(projectDir, 'vibe-dashboard', 'pnpm-workspace.yaml');
  const pnpmWorkspaceFile = fs
    .readFileSync(pnpmWorkspacePath, 'utf8')
    .replace(
      '"vite": "npm:@voidzero-dev/vite-plus-core"',
      `"vite": "file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz"`,
    )
    .replace(
      '"vitest": "npm:@voidzero-dev/vite-plus-test"',
      `"vitest": "file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz"
  "@vitest/browser": "file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz"
  "@vitest/browser-playwright": "file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz"
  "@voidzero-dev/vite-plus": "file:${tgzPath}/voidzero-dev-vite-plus-0.0.0.tgz"
  "@voidzero-dev/vite-plus-core": "file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz"
  "@voidzero-dev/vite-plus-test": "file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz"`,
    );
  fs.writeFileSync(pnpmWorkspacePath, pnpmWorkspaceFile);

  // Remove @vitest/* packages from apps/dashboard/package.json
  // These are bundled into our vitest package and shouldn't be installed separately
  const dashboardPackageJsonPath = join(
    projectDir,
    'vibe-dashboard',
    'apps',
    'dashboard',
    'package.json',
  );
  const dashboardPackageJson = JSON.parse(fs.readFileSync(dashboardPackageJsonPath, 'utf8'));
  if (dashboardPackageJson.devDependencies) {
    // Remove @vitest/browser, @vitest/ui, and @vitest/browser-playwright
    // They're all bundled in our vitest package now
    const vitestPackagesToRemove = ['@vitest/browser', '@vitest/ui', '@vitest/browser-playwright'];
    for (const pkg of vitestPackagesToRemove) {
      delete dashboardPackageJson.devDependencies[pkg];
    }
  }

  // Note: @vitest/* packages are now bundled into our vitest package, so we don't need
  // to add them as separate devDependencies anymore.

  // Write the updated package.json
  fs.writeFileSync(dashboardPackageJsonPath, JSON.stringify(dashboardPackageJson, null, 2) + '\n');

  // Update vite.config.ts to import from vitest/browser-playwright instead of @vitest/browser-playwright
  // This is needed because pnpm overrides don't affect Node.js module resolution at config load time
  const viteConfigPath = join(projectDir, 'vibe-dashboard', 'apps', 'dashboard', 'vite.config.ts');
  const viteConfigContent = fs
    .readFileSync(viteConfigPath, 'utf8')
    .replace('from "@vitest/browser-playwright"', 'from "vitest/browser-playwright"');
  fs.writeFileSync(viteConfigPath, viteConfigContent);

  // Add pnpm overrides to ensure @vitest/* packages are installed at matching versions
  const vitestVersion = vitestPackageJson.devDependencies['@vitest/runner'];
  const vitestOverrides = [
    '@vitest/runner',
    '@vitest/utils',
    '@vitest/spy',
    '@vitest/expect',
    '@vitest/snapshot',
    '@vitest/mocker',
    '@vitest/pretty-format',
  ];

  const pnpmWorkspaceContent = fs.readFileSync(pnpmWorkspacePath, 'utf8');
  if (!pnpmWorkspaceContent.includes('"@vitest/runner":')) {
    const overridesStr = vitestOverrides.map((pkg) => `  "${pkg}": "${vitestVersion}"`).join('\n');
    const updatedContent = pnpmWorkspaceContent.replace(
      /^overrides:\n/m,
      `overrides:\n${overridesStr}\n`,
    );
    fs.writeFileSync(pnpmWorkspacePath, updatedContent);
  }
}

async function patchSkeleton() {
  const packageJsonPath = join(projectDir, 'skeleton', 'package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

  // Change test command from "vitest run" to "vite test"
  packageJson.scripts.test = 'vite test';

  // Add pnpm overrides with tgz files
  // Include @vitest/browser and @vitest/browser-playwright to use bundled versions
  packageJson.pnpm = packageJson.pnpm || {};
  packageJson.pnpm.overrides = {
    ...packageJson.pnpm.overrides,
    vite: `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    'rolldown-vite': `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    vitest: `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
    '@vitest/browser': `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
    '@vitest/browser-playwright': `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
    '@voidzero-dev/vite-plus': `file:${tgzPath}/voidzero-dev-vite-plus-0.0.0.tgz`,
    '@voidzero-dev/vite-plus-core': `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    '@voidzero-dev/vite-plus-test': `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
  };

  packageJson.devDependencies = {
    ...packageJson.devDependencies,
    '@voidzero-dev/vite-plus': `latest`,
    playwright: `catalog:`,
  };

  // Relax engine constraints to support broader node versions
  if (packageJson.engines?.node) {
    packageJson.engines.node = '>=22.0.0';
  }

  fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');

  const vitestVersion = vitestPackageJson.devDependencies['@vitest/expect'];

  // Patch pnpm-workspace.yaml
  const pnpmWorkspacePath = join(projectDir, 'skeleton', 'pnpm-workspace.yaml');
  let pnpmWorkspaceContent = fs
    .readFileSync(pnpmWorkspacePath, 'utf8')
    .replace(`trustPolicy: no-downgrade`, '\n')
    .replace(
      /'@vitest\/browser-playwright': [\d.]+/,
      `'@vitest/browser-playwright': ${vitestVersion}`,
    );

  // Add entries to existing minimumReleaseAgeExclude if it exists, otherwise append new section
  const newExcludes = ["'@voidzero-dev/*'", "'@vitest/*'", 'oxlint', 'oxfmt', 'oxlint-tsgolint'];
  if (pnpmWorkspaceContent.includes('minimumReleaseAgeExclude:')) {
    // Find the minimumReleaseAgeExclude section and add new entries
    pnpmWorkspaceContent = pnpmWorkspaceContent.replace(
      /minimumReleaseAgeExclude:\n((?:  - .+\n)+)/,
      (_, existingEntries) => {
        const newEntriesStr = newExcludes.map((e) => `  - ${e}\n`).join('');
        return `minimumReleaseAgeExclude:\n${existingEntries}${newEntriesStr}`;
      },
    );
  } else {
    pnpmWorkspaceContent += `\nminimumReleaseAgeExclude:\n${newExcludes.map((e) => `  - ${e}`).join('\n')}\n`;
  }

  // Add peerDependencyRules if not present
  if (!pnpmWorkspaceContent.includes('peerDependencyRules:')) {
    pnpmWorkspaceContent += `
peerDependencyRules:
  allowAny:
    - vite
    - vitest
`;
  }

  fs.writeFileSync(pnpmWorkspacePath, pnpmWorkspaceContent);

  // Update vite.config.ts files to import from vitest/browser-playwright instead of @vitest/browser-playwright
  // This is needed because pnpm overrides don't affect Node.js module resolution at config load time
  const skeletonReactConfigPath = join(
    projectDir,
    'skeleton',
    'packages',
    'skeleton-react',
    'vite.config.ts',
  );
  const skeletonSvelteConfigPath = join(
    projectDir,
    'skeleton',
    'packages',
    'skeleton-svelte',
    'vite.config.ts',
  );

  for (const configPath of [skeletonReactConfigPath, skeletonSvelteConfigPath]) {
    const content = fs
      .readFileSync(configPath, 'utf8')
      // Handle both single and double quotes
      .replace(/from ['"]@vitest\/browser-playwright['"]/, 'from "vitest/browser-playwright"');
    fs.writeFileSync(configPath, content);
  }

  // Remove @vitest/browser-playwright from package devDependencies
  // These are bundled in our vitest package now
  const packagesToUpdate = [
    join(projectDir, 'skeleton', 'packages', 'skeleton-react', 'package.json'),
    join(projectDir, 'skeleton', 'packages', 'skeleton-svelte', 'package.json'),
  ];

  for (const pkgPath of packagesToUpdate) {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
    if (pkg.devDependencies?.['@vitest/browser-playwright']) {
      delete pkg.devDependencies['@vitest/browser-playwright'];
    }
    fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, '\t') + '\n');
  }
}

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
}

switch (project) {
  case 'vibe-dashboard':
    await patchVibeDashboard();
    break;
  case 'skeleton':
    await patchSkeleton();
    break;
  case 'rollipop':
    await migrateProject(project);
    break;
  default:
    console.error(`Project ${project} is not supported`);
    process.exit(1);
}

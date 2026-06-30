import { execSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { VITEST_VERSION } from '../packages/cli/src/utils/constants.ts';
import { ecosystemCiDir, tgzDir } from './paths.ts';
import repos from './repo.json' with { type: 'json' };

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const repoRoot = join(ecosystemCiDir, project);
const repoConfig = repos[project as keyof typeof repos];
const directory = 'directory' in repoConfig ? repoConfig.directory : undefined;
const cwd = directory ? join(repoRoot, directory) : repoRoot;
// The e2e build job pins packages/cli to 0.0.0 before `pnpm pack`, so the
// artifact is always vite-plus-0.0.0.tgz regardless of the committed version.
const vitePlusTgz = `file:${tgzDir}/vite-plus-0.0.0.tgz`;
// run vp migrate
const cli = process.env.VP_CLI_BIN ?? 'vp';

if (project === 'rollipop') {
  const oxfmtrc = await readFile(join(repoRoot, '.oxfmtrc.json'), 'utf-8');
  await writeFile(
    join(repoRoot, '.oxfmtrc.json'),
    oxfmtrc.replace('      ["ts-equals-import"],\n', ''),
    'utf-8',
  );
}

if (project === 'vinext') {
  // vinext sets `minimumReleaseAge` (24h) which blocks fresh upstream upgrades
  // (e.g. oxc 0.129.0 published <24h ago). Disable it for the ecosystem run so
  // upgrade-deps PRs can install transitive deps that were just published.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf-8');
  const patched = workspace.replace(/^minimumReleaseAge:.*$/m, 'minimumReleaseAge: 0');
  if (patched === workspace) {
    throw new Error(`vinext patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  await writeFile(workspacePath, patched, 'utf-8');

  // The single in-process `integration` project runs serially and its ISR
  // revalidation test sits right at the 30s ceiling under CI load (observed
  // 26.8s on green main runs, 30.0s here) — a borderline timeout, not a real
  // regression (the vitest runner is byte-identical across this bump). Give it
  // headroom so the ecosystem run isn't flaky.
  const viteConfigPath = join(repoRoot, 'vite.config.ts');
  const viteConfig = await readFile(viteConfigPath, 'utf-8');
  const patchedConfig = viteConfig.replace('testTimeout: 30000', 'testTimeout: 60000');
  if (patchedConfig === viteConfig) {
    throw new Error(`vinext patch: \`testTimeout: 30000\` not found in ${viteConfigPath}`);
  }
  await writeFile(viteConfigPath, patchedConfig, 'utf-8');
}

if (project === 'dify') {
  // dify sets `minimumReleaseAge` (0) with `resolutionMode: time-based`, and
  // pnpm 11.5.2 crashes with ERR_PNPM_RESOLUTION_POLICY_VIOLATIONS_UNHANDLED
  // once the policy machinery is active and the local `file:` tgz overrides
  // produce violations (file deps have no publish timestamp). Remove the key
  // so the policy stays inactive for the ecosystem run.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf-8');
  const patched = workspace.replace(/^minimumReleaseAge:.*\n/m, '');
  if (patched === workspace) {
    throw new Error(`dify patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  await writeFile(workspacePath, patched, 'utf-8');
}

// Projects that already use vite-plus need VP_FORCE_MIGRATE=1 so
// vp migrate runs full dependency rewriting instead of skipping.
const forceFreshMigration = 'forceFreshMigration' in repoConfig && repoConfig.forceFreshMigration;

// Bun is uniquely strict about vitest's `peer vite ^6 || ^7 || ^8` resolution
// (https://github.com/oven-sh/bun/issues/8406): it checks both the override
// target's package name and version. Point bun-based projects at the
// vite-7.99.0 alias tgz (a copy of core renamed to "vite" with a satisfying
// version); pnpm/npm/yarn must keep pointing at the real core tgz, otherwise
// they trip a registry lookup for "vite@<version>" when a workspace
// sub-package and the override both reference the same vite-named alias.
const isBunProject = project === 'bun-vite-template';
const viteOverrideTgz = isBunProject ? `vite-7.99.0.tgz` : `voidzero-dev-vite-plus-core-0.0.0.tgz`;

// Mirror VITE_PLUS_OVERRIDE_PACKAGES: pin `vitest` only. The `@vitest/*` family
// are exact deps of `vitest`, so a single `vitest` override cascades them.
//
// Coverage providers are intentionally NOT in the shipped override map (the
// product leaves them user-owned; the runtime guard fail-fasts on a skew). But
// this rig FORCE-INSTALLS the locally built vitest, and many ecosystem projects
// pin an older `@vitest/coverage-*` in their lockfile. Without alignment, the
// forced runner (4.1.9) skews from the project's pinned provider and the guard
// aborts `vp test --coverage` — testing an incoherent combo no real install has.
// Pin the providers here so the E2E coverage step runs against a consistent
// runner+provider pair, exactly as a user who followed the guard's advice would.
const vitestOverrides = {
  vitest: VITEST_VERSION,
  '@vitest/coverage-v8': VITEST_VERSION,
  '@vitest/coverage-istanbul': VITEST_VERSION,
};

const migrateEnv: NodeJS.ProcessEnv = {
  ...process.env,
  ...(forceFreshMigration ? { VP_FORCE_MIGRATE: '1' } : {}),
  VP_OVERRIDE_PACKAGES: JSON.stringify({
    vite: `file:${tgzDir}/${viteOverrideTgz}`,
    '@voidzero-dev/vite-plus-core': `file:${tgzDir}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    ...vitestOverrides,
  }),
  VP_VERSION: vitePlusTgz,
};

if (project === 'dify') {
  // dify needs the minimumReleaseAge gate disabled so `vp migrate`'s
  // `@oxlint/migrate` dlx can install a same-day toolchain publish. But with
  // the gate active, dify's `resolutionMode: time-based` policy plus the local
  // `file:` tgz overrides (no publish timestamp) trip
  // ERR_PNPM_RESOLUTION_POLICY_VIOLATIONS_UNHANDLED in vp's bundled pnpm during
  // migrate's auto-install. Scope the gate var to this migrate subprocess only,
  // and tolerate the auto-install crash: migrate still generates the config and
  // import rewrites, and the workflow's follow-up `vp install` (run without the
  // var, so the policy stays inactive) does the real install.
  migrateEnv.pnpm_config_minimum_release_age = '0';
  try {
    execSync(`${cli} migrate --no-agent --no-interactive`, {
      cwd,
      stdio: 'inherit',
      env: migrateEnv,
    });
  } catch {
    console.warn(
      'dify: `vp migrate` auto-install failed (expected with the age gate active); ' +
        'continuing, `vp install` will resync node_modules.',
    );
  }
} else {
  execSync(`${cli} migrate --no-agent --no-interactive`, {
    cwd,
    stdio: 'inherit',
    env: migrateEnv,
  });
}

const packageJsonPath = join(cwd, 'package.json');
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf-8')) as {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
};

if (packageJson.dependencies?.['vite-plus']) {
  packageJson.dependencies['vite-plus'] = vitePlusTgz;
} else {
  packageJson.devDependencies ??= {};
  packageJson.devDependencies['vite-plus'] = vitePlusTgz;
}

await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`, 'utf-8');

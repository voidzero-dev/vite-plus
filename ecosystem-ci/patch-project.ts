import { execSync, spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { appendFile, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { VITEST_VERSION } from '../packages/cli/src/utils/constants.ts';
import { ecosystemCiDir, tgzDir, vitePlusTgzVersion } from './paths.ts';
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
// run vp migrate
const cli = process.env.VP_CLI_BIN ?? 'vp';

// The packed local build in tmp/tgz is served through a local npm registry
// (local-npm-registry.ts), so vp migrate pins and installs the checkout's
// own version through the standard registry code paths, with no `file:` specs.
const vitePlusVersion = vitePlusTgzVersion();

const registryScript = join(
  import.meta.dirname,
  '..',
  'packages',
  'tools',
  'src',
  'local-npm-registry.ts',
);
// Detach the server so it can outlive this script on CI: the lockfiles
// written below reference its tarball URLs, and later workflow steps (the
// project's own vp commands) inherit the registry env via GITHUB_ENV.
// stderr must not inherit this process's streams: the detached server would
// hold the step's output pipe open after this script exits.
const registryServer = spawn(
  process.execPath,
  [registryScript, '--serve', '--packages-dir', tgzDir],
  {
    stdio: ['ignore', 'pipe', 'ignore'],
    detached: true,
  },
);
const registryInfo = await new Promise<{ registry: string; env: Record<string, string> }>(
  (resolve, reject) => {
    let buffered = '';
    registryServer.stdout.on('data', (chunk: Buffer) => {
      buffered += chunk.toString();
      const newline = buffered.indexOf('\n');
      if (newline !== -1) {
        resolve(JSON.parse(buffered.slice(0, newline)));
      }
    });
    registryServer.on('error', reject);
    registryServer.on('exit', (code) => reject(new Error(`registry exited early (${code})`)));
  },
);
console.log(
  `Serving local Vite+ packages at ${registryInfo.registry} (vite-plus@${vitePlusVersion})`,
);
// The server prints nothing after the handshake; release the pipe and the
// process handle so they don't keep this script's event loop alive after the
// installs below finish.
registryServer.stdout.destroy();
registryServer.unref();

if (process.env.GITHUB_ENV) {
  // Keep the registry reachable for the workflow's later steps.
  const lines = Object.entries(registryInfo.env)
    .map(([key, value]) => `${key}=${value}\n`)
    .join('');
  await appendFile(process.env.GITHUB_ENV, lines);
} else {
  process.on('exit', () => registryServer.kill());
}

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
  // dify sets `minimumReleaseAge` with `resolutionMode: time-based`. Keep the
  // policy inactive for the ecosystem run so a same-day upstream publish does
  // not fail resolution (the local Vite+ packages themselves carry an old
  // `time` from the registry, so they always pass age gates).
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

// E2E intentionally installs just-published toolchain packages (e.g.
// @oxlint/migrate during `vp migrate`, freshly bumped @oxc-project/runtime
// during `vp install`). Disable pnpm's minimumReleaseAge gate so a same-day
// publish does not fail with ERR_PNPM_NO_MATURE_MATCHING_VERSION. pnpm >= 10.6
// only reads the PNPM_CONFIG_* spelling; older pnpm reads the lowercase form.
//
// Projects with `resolutionMode: time-based` (currently dify) are the
// exception: defining a minimumReleaseAge (even 0, via any env spelling)
// activates pnpm's resolution-policy engine there, which vp's bundled pnpm
// cannot handle (ERR_PNPM_RESOLUTION_POLICY_VIOLATIONS_UNHANDLED, no
// handleResolutionPolicyViolations callback wired). Their `minimumReleaseAge:`
// key is stripped by the per-project patches above, so with no gate env the
// policy stays inactive and installs work.
const workspaceYamlPath = join(repoRoot, 'pnpm-workspace.yaml');
const timeBasedResolution =
  existsSync(workspaceYamlPath) &&
  /^resolutionMode:\s*time-based/m.test(readFileSync(workspaceYamlPath, 'utf-8'));
const releaseAgeEnv = timeBasedResolution
  ? {}
  : {
      pnpm_config_minimum_release_age: '0',
      PNPM_CONFIG_MINIMUM_RELEASE_AGE: '0',
    };

const migrateEnv: NodeJS.ProcessEnv = {
  ...process.env,
  ...registryInfo.env,
  ...(forceFreshMigration ? { VP_FORCE_MIGRATE: '1' } : {}),
  VP_OVERRIDE_PACKAGES: JSON.stringify({
    vite: `npm:@voidzero-dev/vite-plus-core@${vitePlusVersion}`,
    ...vitestOverrides,
  }),
  // The vp binary was built before the pack step pinned the package versions,
  // so align the version migrate pins with the tgz the registry serves.
  VP_VERSION: vitePlusVersion,
  ...releaseAgeEnv,
};

execSync(`${cli} migrate --no-agent --no-interactive`, {
  cwd,
  stdio: 'inherit',
  env: migrateEnv,
});

// Install through the local registry. `vp migrate` already pinned
// `vite-plus@<version>` in package.json exactly like a real migration, so no
// manual package.json rewrite is needed.
execSync(`${cli} install --no-frozen-lockfile`, {
  cwd,
  stdio: 'inherit',
  env: { ...process.env, ...registryInfo.env, ...releaseAgeEnv },
});
